#pragma once

#include <QtGlobal>
#include <optional>

// Stateful low-battery latch for a single battery source (one pod or
// the case). Asymmetric hysteresis prevents repeated notifications
// when a source hovers near the threshold on noisy battery readings:
// trips once at level <= triggerLevel, clears the latch only when
// level rises back to >= resetLevel. Charging clears the latch
// immediately so a brief plug-pull near the threshold doesn't
// lock out future warnings.
//
// evaluate() returns the source's level percent when a notification
// SHOULD fire this update, or std::nullopt when no transition
// occurred. Callers format the user-visible message — the latch
// only owns the trip state.
class LowBatteryLatch {
public:
    constexpr explicit LowBatteryLatch(quint8 trigger = 10, quint8 reset = 15)
        : m_trigger(trigger), m_reset(reset) {}

    std::optional<quint8> evaluate(quint8 level, bool available, bool charging) {
        if (!available) return std::nullopt;
        if (charging) {
            m_notified = false;
            return std::nullopt;
        }
        if (level <= m_trigger && !m_notified) {
            m_notified = true;
            return level;
        }
        if (level >= m_reset) {
            m_notified = false;
        }
        return std::nullopt;
    }

    bool notified() const { return m_notified; }
    quint8 trigger() const { return m_trigger; }
    quint8 reset() const { return m_reset; }

    void resetLatch() { m_notified = false; }

private:
    quint8 m_trigger;
    quint8 m_reset;
    bool m_notified = false;
};
