#pragma once

#include <QString>
#include <QStringView>
#include <optional>

namespace OpenPods::Ipc
{
    // Parse a "prefix:N" IPC verb into the integer payload.
    //
    // Returns std::nullopt when:
    //   - msg doesn't start with prefix
    //   - the payload is empty
    //   - the payload doesn't parse as a base-10 integer
    //   - the parsed integer is outside [min, max] (inclusive)
    //
    // Used by main.cpp's QLocalServer dispatch for the `adaptive:N`
    // family of verbs. Extracted to a header so the parser can be
    // unit-tested without a full daemon spin-up. Future verbs in the
    // same shape (e.g. `mic:N`, `eq:N`) reuse this directly.
    inline std::optional<int> parseIntVerb(QStringView msg,
                                           QStringView prefix,
                                           int min = 0,
                                           int max = 100)
    {
        if (!msg.startsWith(prefix)) {
            return std::nullopt;
        }
        const QStringView payload = msg.mid(prefix.size());
        if (payload.isEmpty()) {
            return std::nullopt;
        }
        // Reject leading whitespace — QString::toInt silently strips
        // it, which masks client-side malformed input like
        // `adaptive: 50`. The dispatch never produces whitespace
        // itself, so any whitespace is a bug worth surfacing.
        if (payload.front().isSpace()) {
            return std::nullopt;
        }
        bool ok = false;
        const int value = payload.toString().toInt(&ok, 10);
        if (!ok) {
            return std::nullopt;
        }
        if (value < min || value > max) {
            return std::nullopt;
        }
        return value;
    }
}
