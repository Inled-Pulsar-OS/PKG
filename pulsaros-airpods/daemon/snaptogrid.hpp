#pragma once

#include <algorithm>

// Pure integer snap-to-grid helper. Used to align audio volume values
// to a fixed step (default 5%) so the visible volume always lands on a
// round number, regardless of which surface produced the change:
//
//   - Quickshell keyboard volume keys (already step-5%)
//   - AirPods stem-swipe (AVRCP AbsoluteVolume 15-step = ~6.67%/swipe)
//   - PulseAudio CA-duck (initialVolume * 0.20, arbitrary fraction)
//
// Snaps `value` to the nearest multiple of `step` using round-half-up
// semantics, then clamps to [0, 100]. step must be >= 1; values <= 0
// fall back to step=1 (no rounding, just the clamp). step is hard-
// capped to 100 to keep the math sane for sliders bound to [0, 100].
//
// Tested exhaustively in tst_snaptogrid.cpp.
inline int snapToGrid(int value, int step = 5)
{
    if (step <= 0) step = 1;
    if (step > 100) step = 100;
    // Half-up rounding done in integer space: shift +step/2 before the
    // integer division so 7 -> 5 -> 5, 8 -> 5 -> 10 with step=5.
    int clamped = std::clamp(value, 0, 100);
    int snapped = ((clamped + step / 2) / step) * step;
    return std::clamp(snapped, 0, 100);
}
