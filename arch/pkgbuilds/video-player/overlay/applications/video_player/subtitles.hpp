// External subtitles: SubRip (.srt) and WebVTT (.vtt), in any common encoding.
#pragma once

#include <string>
#include <vector>

struct SubtitleCue {
    double start = 0;  ///< seconds
    double end = 0;
    std::vector<std::string> lines;  ///< UTF-8, markup removed
    bool italic = false;
};

class SubtitleTrack {
public:
    /// Parses a .srt or .vtt file. Returns false with a message if nothing could be read.
    bool load(const std::string& path, std::string& error);
    /// Parses text already in memory (tests, pasted subtitles).
    void parse(const std::string& text);

    [[nodiscard]] const std::string& path() const { return m_path; }
    /// "Turkish", "English (2)", or the file name when the language can't be told from it.
    [[nodiscard]] const std::string& name() const { return m_name; }
    void setName(std::string name) { m_name = std::move(name); }
    [[nodiscard]] const std::vector<SubtitleCue>& cues() const { return m_cues; }
    /// Cues showing at `t` seconds, in start order.
    [[nodiscard]] std::vector<const SubtitleCue*> at(double t) const;

private:
    std::string m_path;
    std::string m_name;
    std::vector<SubtitleCue> m_cues;
    double m_longest = 0;  // bounds the backwards search in at()
};

/// Subtitle files next to a video: "Movie.srt", "Movie.en.srt", "Movie.Turkish.vtt", ... (same base name).
std::vector<std::string> findSubtitlesFor(const std::string& videoPath);
[[nodiscard]] bool isSubtitleFile(const std::string& path);

/// "tr" / "tur" -> "Turkish"; longer names are returned as they are.
std::string languageName(const std::string& code);
/// The user's language as languageName() spells it (from LC_ALL / LC_MESSAGES / LANG).
std::string userLanguageName();

/// Converts subtitle bytes to UTF-8: BOMs and UTF-16 are honoured, and text that isn't valid UTF-8 is read in
/// the Windows code page of the user's language (1254 for Turkish, 1251 for Cyrillic, ...).
std::string toUtf8(const std::string& raw);
