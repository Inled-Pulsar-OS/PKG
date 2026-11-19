#include "mayonnaise/settings.hpp"

#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <sstream>

namespace fs = std::filesystem;

namespace mayo {

namespace {

std::string configHome() {
    if (const char* x = std::getenv("XDG_CONFIG_HOME"); x && *x == '/') return x;
    const char* home = std::getenv("HOME");
    return std::string(home && *home ? home : ".") + "/.config";
}

/// App names become directory names: keep them to one harmless path component.
std::string sanitize(std::string name) {
    for (char& c : name) {
        if (c == '/' || c == '\\' || static_cast<unsigned char>(c) < 0x20) c = '_';
    }
    if (name.empty() || name == "." || name == "..") name = "app";
    return name;
}

std::string trim(std::string_view s) {
    size_t a = s.find_first_not_of(" \t\r");
    if (a == std::string_view::npos) return {};
    size_t b = s.find_last_not_of(" \t\r");
    return std::string(s.substr(a, b - a + 1));
}

// Values are single lines; newlines and backslashes are escaped.
std::string escape(std::string_view v) {
    std::string out;
    for (char c : v) {
        if (c == '\\') out += "\\\\";
        else if (c == '\n') out += "\\n";
        else out.push_back(c);
    }
    return out;
}

std::string unescape(std::string_view v) {
    std::string out;
    for (size_t i = 0; i < v.size(); ++i) {
        if (v[i] == '\\' && i + 1 < v.size()) {
            ++i;
            out.push_back(v[i] == 'n' ? '\n' : v[i]);
        } else {
            out.push_back(v[i]);
        }
    }
    return out;
}

}  // namespace

Settings::Settings(std::string appName) : m_app(sanitize(std::move(appName))) {
    m_dir = configHome() + "/mayonnaise/" + m_app;
    load();
}

Settings::~Settings() { save(); }

std::string Settings::filePath() const { return m_dir + "/settings.conf"; }

void Settings::load() {
    std::ifstream in(filePath());
    std::string line;
    while (std::getline(in, line)) {
        std::string t = trim(line);
        if (t.empty() || t[0] == '#') continue;
        size_t eq = t.find('=');
        if (eq == std::string::npos) continue;
        std::string key = trim(std::string_view(t).substr(0, eq));
        if (!key.empty()) m_values[key] = unescape(trim(std::string_view(t).substr(eq + 1)));
    }
}

void Settings::save() {
    if (!m_dirty || m_readOnly) return;
    m_dirty = false;
    std::error_code ec;
    fs::create_directories(m_dir, ec);
    const std::string path = filePath();
    const std::string tmp = path + ".tmp";
    {
        std::ofstream out(tmp, std::ios::trunc);
        if (!out) {
            std::fprintf(stderr, "mayonnaise: can't write %s\n", tmp.c_str());
            return;
        }
        out << "# Settings for " << m_app << ", written by Mayonnaise. Safe to edit while the app is closed.\n";
        for (const auto& [key, value] : m_values) out << key << " = " << escape(value) << '\n';
        if (!out.flush()) {
            std::fprintf(stderr, "mayonnaise: can't write %s\n", tmp.c_str());
            return;
        }
    }
    fs::rename(tmp, path, ec);
    if (ec) std::fprintf(stderr, "mayonnaise: can't save %s: %s\n", path.c_str(), ec.message().c_str());
}

void Settings::changed() {
    m_dirty = true;
    if (m_onChange) m_onChange();
    else save();
}

bool Settings::contains(std::string_view key) const { return m_values.find(key) != m_values.end(); }

std::string Settings::getString(std::string_view key, std::string fallback) const {
    auto it = m_values.find(key);
    return it == m_values.end() ? fallback : it->second;
}

bool Settings::getBool(std::string_view key, bool fallback) const {
    auto it = m_values.find(key);
    if (it == m_values.end()) return fallback;
    const std::string& v = it->second;
    if (v == "true" || v == "yes" || v == "on" || v == "1") return true;
    if (v == "false" || v == "no" || v == "off" || v == "0") return false;
    return fallback;
}

double Settings::getDouble(std::string_view key, double fallback) const {
    auto it = m_values.find(key);
    if (it == m_values.end()) return fallback;
    // Locale independent: settings files must read the same everywhere.
    std::istringstream in(it->second);
    in.imbue(std::locale::classic());
    double v = 0;
    return (in >> v) ? v : fallback;
}

int Settings::getInt(std::string_view key, int fallback) const {
    const double v = getDouble(key, fallback);
    return v >= -2147483648.0 && v <= 2147483647.0 ? static_cast<int>(v) : fallback;
}

float Settings::getFloat(std::string_view key, float fallback) const {
    return static_cast<float>(getDouble(key, fallback));
}

void Settings::set(std::string_view key, std::string_view value) {
    auto it = m_values.find(key);
    if (it != m_values.end() && it->second == value) return;
    m_values[std::string(key)] = std::string(value);
    changed();
}

void Settings::set(std::string_view key, bool value) { set(key, std::string_view(value ? "true" : "false")); }

void Settings::set(std::string_view key, int value) { set(key, std::string_view(std::to_string(value))); }

void Settings::set(std::string_view key, double value) {
    std::ostringstream out;
    out.imbue(std::locale::classic());
    out.precision(9);
    out << value;
    set(key, std::string_view(out.str()));
}

void Settings::remove(std::string_view key) {
    auto it = m_values.find(key);
    if (it == m_values.end()) return;
    m_values.erase(it);
    changed();
}

}  // namespace mayo
