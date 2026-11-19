#pragma once

#include <cstddef>
#include <string>
#include <string_view>

namespace mayo::utf8 {

/// Decodes the code point starting at `i` and advances `i`. Invalid bytes decode as U+FFFD.
inline char32_t next(std::string_view s, size_t& i) {
    auto byte = [&](size_t k) { return static_cast<unsigned char>(s[k]); };
    unsigned char c = byte(i);
    if (c < 0x80) {
        ++i;
        return c;
    }
    int extra = (c >= 0xF0) ? 3 : (c >= 0xE0) ? 2 : (c >= 0xC0) ? 1 : -1;
    if (extra < 0 || i + static_cast<size_t>(extra) >= s.size()) {
        ++i;
        return 0xFFFD;
    }
    char32_t cp = c & (0x3F >> extra);
    for (int k = 1; k <= extra; ++k) {
        unsigned char cc = byte(i + static_cast<size_t>(k));
        if ((cc & 0xC0) != 0x80) {
            i += static_cast<size_t>(k);
            return 0xFFFD;
        }
        cp = (cp << 6) | (cc & 0x3F);
    }
    i += static_cast<size_t>(extra) + 1;
    return cp;
}

inline bool isContinuation(char c) { return (static_cast<unsigned char>(c) & 0xC0) == 0x80; }

/// Byte index of the code point boundary before `i`.
inline size_t prevBoundary(std::string_view s, size_t i) {
    if (i == 0) return 0;
    --i;
    while (i > 0 && isContinuation(s[i])) --i;
    return i;
}

/// Byte index of the code point boundary after `i`.
inline size_t nextBoundary(std::string_view s, size_t i) {
    if (i >= s.size()) return s.size();
    ++i;
    while (i < s.size() && isContinuation(s[i])) ++i;
    return i;
}

inline void append(std::string& out, char32_t cp) {
    if (cp < 0x80) {
        out += static_cast<char>(cp);
    } else if (cp < 0x800) {
        out += static_cast<char>(0xC0 | (cp >> 6));
        out += static_cast<char>(0x80 | (cp & 0x3F));
    } else if (cp < 0x10000) {
        out += static_cast<char>(0xE0 | (cp >> 12));
        out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
        out += static_cast<char>(0x80 | (cp & 0x3F));
    } else {
        out += static_cast<char>(0xF0 | (cp >> 18));
        out += static_cast<char>(0x80 | ((cp >> 12) & 0x3F));
        out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
        out += static_cast<char>(0x80 | (cp & 0x3F));
    }
}

inline bool isWordChar(char32_t cp) {
    return (cp >= '0' && cp <= '9') || (cp >= 'a' && cp <= 'z') || (cp >= 'A' && cp <= 'Z') || cp == '_' || cp >= 0x80;
}

}  // namespace mayo::utf8
