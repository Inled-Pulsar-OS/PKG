#include "subtitles.hpp"

#include <iconv.h>

#include <algorithm>
#include <cctype>
#include <cerrno>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <map>
#include <sstream>

namespace fs = std::filesystem;

namespace {

std::string lower(std::string s) {
    for (auto& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return s;
}

bool validUtf8(const std::string& s) {
    size_t i = 0;
    while (i < s.size()) {
        const auto c = static_cast<unsigned char>(s[i]);
        size_t n = c < 0x80 ? 1 : (c >> 5) == 0x6 ? 2 : (c >> 4) == 0xE ? 3 : (c >> 3) == 0x1E ? 4 : 0;
        if (n == 0 || i + n > s.size()) return false;
        for (size_t k = 1; k < n; ++k) {
            if ((static_cast<unsigned char>(s[i + k]) & 0xC0) != 0x80) return false;
        }
        i += n;
    }
    return true;
}

std::string convert(const std::string& raw, const char* from) {
    iconv_t cd = iconv_open("UTF-8", from);
    if (cd == reinterpret_cast<iconv_t>(-1)) return raw;
    std::string out(raw.size() * 4 + 16, '\0');
    char* in = const_cast<char*>(raw.data());
    size_t inLeft = raw.size();
    char* dst = out.data();
    size_t outLeft = out.size();
    while (inLeft > 0) {
        if (iconv(cd, &in, &inLeft, &dst, &outLeft) == static_cast<size_t>(-1)) {
            if (errno == EILSEQ || errno == EINVAL) {
                ++in;  // skip a byte the code page doesn't define
                --inLeft;
                continue;
            }
            break;
        }
    }
    iconv_close(cd);
    out.resize(out.size() - outLeft);
    return out;
}

/// The legacy code page subtitle files in the user's language are usually saved in.
const char* legacyCodepage() {
    const char* env = std::getenv("LC_ALL");
    if (!env || !*env) env = std::getenv("LC_MESSAGES");
    if (!env || !*env) env = std::getenv("LANG");
    const std::string lang = env ? std::string(env).substr(0, 2) : "";
    static const std::map<std::string, const char*> kPages = {
        {"tr", "WINDOWS-1254"}, {"az", "WINDOWS-1254"}, {"ru", "WINDOWS-1251"}, {"uk", "WINDOWS-1251"},
        {"bg", "WINDOWS-1251"}, {"sr", "WINDOWS-1251"}, {"mk", "WINDOWS-1251"}, {"be", "WINDOWS-1251"},
        {"pl", "WINDOWS-1250"}, {"cs", "WINDOWS-1250"}, {"sk", "WINDOWS-1250"}, {"hu", "WINDOWS-1250"},
        {"ro", "WINDOWS-1250"}, {"hr", "WINDOWS-1250"}, {"sl", "WINDOWS-1250"}, {"el", "WINDOWS-1253"},
        {"he", "WINDOWS-1255"}, {"ar", "WINDOWS-1256"}, {"fa", "WINDOWS-1256"}, {"lt", "WINDOWS-1257"},
        {"lv", "WINDOWS-1257"}, {"et", "WINDOWS-1257"}, {"vi", "WINDOWS-1258"}, {"th", "TIS-620"},
        {"zh", "GB18030"},      {"ja", "SHIFT_JIS"},    {"ko", "EUC-KR"},
    };
    auto it = kPages.find(lang);
    return it != kPages.end() ? it->second : "WINDOWS-1252";
}

/// "01:02:03,456", "02:03.456" or "1:02:03.4" -> seconds; negative when malformed.
double parseTime(const std::string& s) {
    double parts[3] = {0, 0, 0};
    int count = 0;
    size_t i = 0;
    while (i < s.size() && count < 3) {
        size_t j = i;
        while (j < s.size() && (std::isdigit(static_cast<unsigned char>(s[j])) || s[j] == ',' || s[j] == '.')) ++j;
        if (j == i) return -1;
        std::string num = s.substr(i, j - i);
        std::replace(num.begin(), num.end(), ',', '.');
        parts[count++] = std::atof(num.c_str());
        if (j >= s.size() || s[j] != ':') break;
        i = j + 1;
    }
    if (count == 3) return parts[0] * 3600 + parts[1] * 60 + parts[2];
    if (count == 2) return parts[0] * 60 + parts[1];
    return -1;
}

std::string trim(const std::string& s) {
    const size_t b = s.find_first_not_of(" \t\r\n");
    if (b == std::string::npos) return {};
    return s.substr(b, s.find_last_not_of(" \t\r\n") - b + 1);
}

/// Removes HTML-ish tags (<i>, <font ...>, <c.yellow>) and ASS override blocks ({\an8}); decodes entities.
std::string stripMarkup(const std::string& line, bool& italic) {
    std::string out;
    for (size_t i = 0; i < line.size(); ++i) {
        if (line[i] == '<') {
            const size_t e = line.find('>', i);
            if (e != std::string::npos) {
                const std::string tag = lower(line.substr(i + 1, e - i - 1));
                if (tag == "i" || tag.rfind("i ", 0) == 0) italic = true;
                i = e;
                continue;
            }
        }
        if (line[i] == '{' && i + 1 < line.size() && line[i + 1] == '\\') {
            const size_t e = line.find('}', i);
            if (e != std::string::npos) {
                if (line.find("\\i1", i) < e) italic = true;
                i = e;
                continue;
            }
        }
        out += line[i];
    }
    for (auto [from, to] : {std::pair{"&amp;", "&"}, {"&lt;", "<"}, {"&gt;", ">"}, {"&nbsp;", " "}, {"&quot;", "\""},
                            {"&#39;", "'"}, {"\\N", " "}, {"\\h", " "}}) {
        for (size_t p; (p = out.find(from)) != std::string::npos;) out.replace(p, std::strlen(from), to);
    }
    return trim(out);
}

}  // namespace

std::string languageName(const std::string& code) {
    static const std::map<std::string, std::string> kNames = {
        {"tr", "Turkish"}, {"tur", "Turkish"}, {"en", "English"}, {"eng", "English"}, {"de", "German"},
        {"ger", "German"}, {"deu", "German"}, {"fr", "French"},  {"fre", "French"},  {"fra", "French"},
        {"es", "Spanish"}, {"spa", "Spanish"}, {"it", "Italian"}, {"ita", "Italian"}, {"pt", "Portuguese"},
        {"por", "Portuguese"}, {"ru", "Russian"}, {"rus", "Russian"}, {"ar", "Arabic"}, {"ara", "Arabic"},
        {"ja", "Japanese"}, {"jpn", "Japanese"}, {"zh", "Chinese"}, {"chi", "Chinese"}, {"ko", "Korean"},
        {"kor", "Korean"}, {"nl", "Dutch"},   {"pl", "Polish"},  {"sv", "Swedish"}, {"el", "Greek"},
        {"he", "Hebrew"},  {"hi", "Hindi"},   {"uk", "Ukrainian"}, {"cs", "Czech"}, {"hu", "Hungarian"},
        {"ro", "Romanian"}, {"fa", "Persian"}, {"az", "Azerbaijani"}, {"forced", "Forced"}, {"sdh", "SDH"},
    };
    auto it = kNames.find(lower(code));
    if (it != kNames.end()) return it->second;
    if (code.size() > 3) return code;  // "Turkish", "English SDH"
    std::string up = code;
    for (auto& c : up) c = static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    return up;
}

std::string userLanguageName() {
    const char* env = std::getenv("LC_ALL");
    if (!env || !*env) env = std::getenv("LC_MESSAGES");
    if (!env || !*env) env = std::getenv("LANG");
    return env && std::strlen(env) >= 2 ? languageName(std::string(env).substr(0, 2)) : "English";
}

std::string toUtf8(const std::string& raw) {
    if (raw.size() >= 3 && raw.compare(0, 3, "\xEF\xBB\xBF") == 0) return raw.substr(3);
    if (raw.size() >= 2 && static_cast<unsigned char>(raw[0]) == 0xFF && static_cast<unsigned char>(raw[1]) == 0xFE)
        return convert(raw.substr(2), "UTF-16LE");
    if (raw.size() >= 2 && static_cast<unsigned char>(raw[0]) == 0xFE && static_cast<unsigned char>(raw[1]) == 0xFF)
        return convert(raw.substr(2), "UTF-16BE");
    if (validUtf8(raw)) return raw;
    return convert(raw, legacyCodepage());
}

bool isSubtitleFile(const std::string& path) {
    const std::string ext = lower(fs::path(path).extension().string());
    return ext == ".srt" || ext == ".vtt";
}

void SubtitleTrack::parse(const std::string& text) {
    m_cues.clear();
    m_longest = 0;
    std::istringstream in(text);
    std::string line;
    SubtitleCue cue;
    bool inCue = false;
    auto finish = [&] {
        if (inCue && !cue.lines.empty() && cue.end > cue.start) {
            m_longest = std::max(m_longest, cue.end - cue.start);
            m_cues.push_back(std::move(cue));
        }
        cue = {};
        inCue = false;
    };
    while (std::getline(in, line)) {
        if (!line.empty() && line.back() == '\r') line.pop_back();
        const size_t arrow = line.find("-->");
        if (arrow != std::string::npos) {
            finish();
            // "00:00:01,000 --> 00:00:04,000" (WebVTT may add cue settings after the end time)
            std::string endPart = trim(line.substr(arrow + 3));
            endPart = endPart.substr(0, endPart.find_first_of(" \t"));
            const double start = parseTime(trim(line.substr(0, arrow)));
            const double end = parseTime(endPart);
            if (start >= 0 && end >= 0) {
                cue.start = start;
                cue.end = end;
                inCue = true;
            }
            continue;
        }
        if (trim(line).empty()) {
            finish();
            continue;
        }
        if (!inCue) continue;  // cue numbers, WEBVTT header, NOTE / STYLE blocks
        bool italic = false;
        std::string clean = stripMarkup(line, italic);
        if (italic) cue.italic = true;
        if (!clean.empty()) cue.lines.push_back(std::move(clean));
    }
    finish();
    std::stable_sort(m_cues.begin(), m_cues.end(), [](const SubtitleCue& a, const SubtitleCue& b) { return a.start < b.start; });
}

bool SubtitleTrack::load(const std::string& path, std::string& error) {
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        error = "The subtitle file couldn't be opened.";
        return false;
    }
    std::stringstream ss;
    ss << in.rdbuf();
    m_path = path;
    parse(toUtf8(ss.str()));
    if (m_cues.empty()) {
        error = "No subtitles were found in “" + fs::path(path).filename().string() + "”.";
        return false;
    }
    // "Movie.tr.srt" -> Turkish; "Movie.srt" -> the file name.
    const std::string stem = fs::path(path).stem().string();
    const size_t dot = stem.rfind('.');
    m_name = dot != std::string::npos ? languageName(stem.substr(dot + 1)) : fs::path(path).filename().string();
    return true;
}

std::vector<const SubtitleCue*> SubtitleTrack::at(double t) const {
    std::vector<const SubtitleCue*> out;
    auto it = std::upper_bound(m_cues.begin(), m_cues.end(), t, [](double v, const SubtitleCue& c) { return v < c.start; });
    // Walk back over cues that started earlier but may still be showing.
    while (it != m_cues.begin()) {
        --it;
        if (t - it->start > m_longest) break;
        if (t < it->end) out.insert(out.begin(), &*it);
    }
    return out;
}

std::vector<std::string> findSubtitlesFor(const std::string& videoPath) {
    std::vector<std::string> out;
    const fs::path video(videoPath);
    const std::string stem = lower(video.stem().string());
    std::error_code ec;
    for (const auto& e : fs::directory_iterator(video.parent_path(), ec)) {
        const std::string p = e.path().string();
        if (!isSubtitleFile(p)) continue;
        const std::string s = lower(e.path().stem().string());
        if (s == stem || s.rfind(stem + ".", 0) == 0) out.push_back(p);
    }
    std::sort(out.begin(), out.end());
    return out;
}
