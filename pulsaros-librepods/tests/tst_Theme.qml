// Theme.qml token regression test.
//
// Catches accidental palette drift: if someone replaces #0d0d0d with
// "#000000" or aliases colCharging away from colOnline, the gate fails
// before merge. Each test asserts the *concrete* hex value plus the
// alias relationships that Theme.qml documents.

import QtQuick 2.15
import QtTest 6.0
import linux 1.0

TestCase {
    name: "Theme"

    function compareHex(actual, expected) {
        // Qt's Color string form is "#ff0d0d0d" (with alpha). Strip and
        // lowercase before comparing to the canonical "#0d0d0d" form.
        var s = actual.toString().toLowerCase();
        if (s.length === 9 && s.startsWith("#ff"))
            s = "#" + s.substring(3);
        compare(s, expected.toLowerCase(), "expected " + expected + " got " + actual);
    }

    function test_canonicalPalette() {
        compareHex(Theme.colBg,        "#0d0d0d");
        compareHex(Theme.colFg,        "#ffffff");
        compareHex(Theme.colDim,       "#8d8d8d");
        compareHex(Theme.colAccent,    "#b6b6b6");
        compareHex(Theme.colBorder,    "#2a2a2a");
        compareHex(Theme.colToggleOn,  "#ffffff");
        compareHex(Theme.colToggleOff, "#1a1a1a");
        compareHex(Theme.colUrgent,    "#a55555");
        compareHex(Theme.colOnline,    "#6fdc6f");
        compareHex(Theme.colWarning,   "#cecece");
    }

    function test_semanticAliases() {
        // These MUST track the canonical palette — they're aliases, not
        // independent values. Catches "I redefined colCharging to a new
        // green by accident".
        compare(Theme.colSurface,  Theme.colToggleOff);
        compare(Theme.colCharging, Theme.colOnline);
        compare(Theme.colBatLow,   Theme.colUrgent);
        compare(Theme.colBatMid,   Theme.colWarning);
        compare(Theme.colBatHigh,  Theme.colOnline);
    }

    function test_metrics() {
        compare(Theme.radius,      12);
        compare(Theme.tileRadius,  8);
        compare(Theme.borderWidth, 1);
        compare(Theme.fontSize,    14);
        compare(Theme.fontFamily,  "SF Pro Text");
        compare(Theme.iconFont,    "JetBrainsMono Nerd Font");
    }

    function test_noLightModeAccidentallyInverted() {
        // Sanity guard: dark mode is non-negotiable. The window bg must
        // be strictly darker than the toggle-off tier, and toggle-off
        // strictly darker than the border tier — failing this means
        // someone inverted to light.
        function brightness(c) {
            return (c.r + c.g + c.b) / 3.0;
        }
        verify(brightness(Theme.colBg)        < brightness(Theme.colToggleOff));
        verify(brightness(Theme.colToggleOff) < brightness(Theme.colBorder));
        verify(brightness(Theme.colFg)        > brightness(Theme.colAccent));
    }
}
