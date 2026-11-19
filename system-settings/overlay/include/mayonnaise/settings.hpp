#pragma once

#include <functional>
#include <map>
#include <string>
#include <string_view>

namespace mayo {

/// Persistent per-application preferences: a small `key = value` file at
/// `$XDG_CONFIG_HOME/mayonnaise/<app>/settings.conf` (normally `~/.config/mayonnaise/<app>/`).
///
/// The App owns one (`App::settings()`) and also stores the appearance, accent colour and remembered window
/// sizes in it. Changes are written shortly after they happen (a slider drag is one write) and when the App
/// exits. Writes are atomic (temporary file + rename), so a crash never leaves a half-written file.
///
///     auto& s = app.settings();
///     bool hidden = s.getBool("showHidden");
///     s.set("showHidden", true);
class Settings {
public:
    explicit Settings(std::string appName);
    ~Settings();  ///< writes pending changes
    Settings(const Settings&) = delete;
    Settings& operator=(const Settings&) = delete;

    [[nodiscard]] bool getBool(std::string_view key, bool fallback = false) const;
    [[nodiscard]] int getInt(std::string_view key, int fallback = 0) const;
    [[nodiscard]] double getDouble(std::string_view key, double fallback = 0) const;
    [[nodiscard]] float getFloat(std::string_view key, float fallback = 0) const;
    [[nodiscard]] std::string getString(std::string_view key, std::string fallback = {}) const;
    [[nodiscard]] bool contains(std::string_view key) const;

    void set(std::string_view key, bool value);
    void set(std::string_view key, int value);
    void set(std::string_view key, double value);
    void set(std::string_view key, float value) { set(key, static_cast<double>(value)); }
    void set(std::string_view key, std::string_view value);
    void set(std::string_view key, const char* value) { set(key, std::string_view(value)); }
    void set(std::string_view key, const std::string& value) { set(key, std::string_view(value)); }
    void remove(std::string_view key);

    /// Writes the file now (only if something changed).
    void save();
    [[nodiscard]] const std::string& appName() const { return m_app; }
    /// `~/.config/mayonnaise/<app>`; apps may keep other files here too.
    [[nodiscard]] const std::string& directory() const { return m_dir; }
    [[nodiscard]] std::string filePath() const;

    /// Runs after every change. The App uses it to schedule a deferred save; without an App every change
    /// is written immediately.
    void setOnChange(std::function<void()> fn) { m_onChange = std::move(fn); }
    /// Keep changes in memory only (screenshot and test runs must not touch the user's settings).
    void setReadOnly(bool readOnly) { m_readOnly = readOnly; }

private:
    void changed();
    void load();

    std::string m_app;
    std::string m_dir;
    std::map<std::string, std::string, std::less<>> m_values;
    bool m_dirty = false;
    bool m_readOnly = false;
    std::function<void()> m_onChange;
};

}  // namespace mayo
