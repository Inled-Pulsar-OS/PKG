#include "json.hpp"

#include <cstdlib>

namespace json {

namespace {
const Value& nullValue() {
    static const Value v;
    return v;
}

void appendUtf8(std::string& out, unsigned cp) {
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
}  // namespace

class Parser {
public:
    explicit Parser(std::string_view s) : m_s(s) {}

    Value value(int depth = 0) {
        Value v;
        skip();
        if (m_i >= m_s.size() || depth > 64) return v;
        const char c = m_s[m_i];
        if (c == '{') {
            ++m_i;
            v.m_type = Value::Type::Object;
            skip();
            if (peek('}')) return ++m_i, v;
            while (m_i < m_s.size()) {
                skip();
                std::string key = string();
                skip();
                if (!peek(':')) break;
                ++m_i;
                v.m_object.insert_or_assign(std::move(key), value(depth + 1));
                skip();
                if (peek(',')) {
                    ++m_i;
                    continue;
                }
                if (peek('}')) ++m_i;
                break;
            }
        } else if (c == '[') {
            ++m_i;
            v.m_type = Value::Type::Array;
            skip();
            if (peek(']')) return ++m_i, v;
            while (m_i < m_s.size()) {
                v.m_array.push_back(value(depth + 1));
                skip();
                if (peek(',')) {
                    ++m_i;
                    continue;
                }
                if (peek(']')) ++m_i;
                break;
            }
        } else if (c == '"') {
            v.m_type = Value::Type::String;
            v.m_string = string();
        } else if (m_s.substr(m_i, 4) == "true") {
            m_i += 4;
            v.m_type = Value::Type::Bool;
            v.m_bool = true;
        } else if (m_s.substr(m_i, 5) == "false") {
            m_i += 5;
            v.m_type = Value::Type::Bool;
        } else if (m_s.substr(m_i, 4) == "null") {
            m_i += 4;
        } else {
            const std::string num(m_s.substr(m_i, 32));
            char* end = nullptr;
            v.m_number = std::strtod(num.c_str(), &end);
            const auto used = static_cast<size_t>(end - num.c_str());
            if (used == 0) {
                ++m_i;  // skip garbage
            } else {
                m_i += used;
                v.m_type = Value::Type::Number;
            }
        }
        return v;
    }

private:
    void skip() {
        while (m_i < m_s.size() && (m_s[m_i] == ' ' || m_s[m_i] == '\n' || m_s[m_i] == '\r' || m_s[m_i] == '\t')) ++m_i;
    }
    bool peek(char c) const { return m_i < m_s.size() && m_s[m_i] == c; }

    std::string string() {
        std::string out;
        if (!peek('"')) return out;
        ++m_i;
        while (m_i < m_s.size() && m_s[m_i] != '"') {
            char c = m_s[m_i++];
            if (c != '\\' || m_i >= m_s.size()) {
                out += c;
                continue;
            }
            const char e = m_s[m_i++];
            switch (e) {
                case 'n': out += '\n'; break;
                case 't': out += '\t'; break;
                case 'r': out += '\r'; break;
                case 'b': out += '\b'; break;
                case 'f': out += '\f'; break;
                case 'u': {
                    unsigned cp = static_cast<unsigned>(std::strtoul(std::string(m_s.substr(m_i, 4)).c_str(), nullptr, 16));
                    m_i += 4;
                    // Surrogate pair.
                    if (cp >= 0xD800 && cp < 0xDC00 && m_s.substr(m_i, 2) == "\\u") {
                        const unsigned lo = static_cast<unsigned>(std::strtoul(std::string(m_s.substr(m_i + 2, 4)).c_str(), nullptr, 16));
                        m_i += 6;
                        cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                    }
                    appendUtf8(out, cp);
                    break;
                }
                default: out += e;
            }
        }
        ++m_i;
        return out;
    }

    std::string_view m_s;
    size_t m_i = 0;
};

const std::string& Value::string() const {
    static const std::string empty;
    return m_type == Type::String ? m_string : empty;
}

const std::vector<Value>& Value::array() const {
    static const std::vector<Value> empty;
    return m_type == Type::Array ? m_array : empty;
}

const std::map<std::string, Value, std::less<>>& Value::object() const {
    static const std::map<std::string, Value, std::less<>> empty;
    return m_type == Type::Object ? m_object : empty;
}

const Value& Value::operator[](std::string_view key) const {
    if (m_type != Type::Object) return nullValue();
    auto it = m_object.find(key);
    return it == m_object.end() ? nullValue() : it->second;
}

Value Value::parse(std::string_view text) { return Parser(text).value(); }

}  // namespace json
