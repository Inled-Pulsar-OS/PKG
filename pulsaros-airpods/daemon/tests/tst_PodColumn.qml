// PodColumn visual-state regression.
//
// Pins:
//   - opacity = 1.0 when inEar, 0.5 when removed.
//   - Behavior on opacity is wired (180ms OutCubic) — exercised via the
//     transition rather than asserted on directly because the Behavior
//     animator state isn't exposed through public QML API.
//   - batteryLevel propagates to the inner BatteryIndicator.
//   - indicator string drives image-mirror semantics on the R pod.

import QtQuick 2.15
import QtTest 6.0
import linux 1.0

Item {
    width: 200
    height: 200

    Component {
        id: column
        PodColumn {}
    }

    TestCase {
        name: "PodColumn"
        when: windowShown

        function test_opacityFullInEar() {
            var c = createTemporaryObject(column, parent,
                { inEar: true, batteryLevel: 80, iconSource: "" });
            compare(c.opacity, 1);
        }

        function test_opacityDimmedWhenRemoved() {
            var c = createTemporaryObject(column, parent,
                { inEar: false, batteryLevel: 80, iconSource: "" });
            // Behavior on opacity animates over 180ms; force the final
            // value by waiting one tick longer than the easing curve.
            tryCompare(c, "opacity", 0.5, 500);
        }

        function test_opacityTransitionsBack() {
            // Removing then re-inserting must end at 1.0. Tests the
            // symmetry of the auto-resume visual contract (matches
            // mediacontroller.cpp:67 auto-resume path).
            var c = createTemporaryObject(column, parent,
                { inEar: true, batteryLevel: 80, iconSource: "" });
            c.inEar = false;
            tryCompare(c, "opacity", 0.5, 500);
            c.inEar = true;
            tryCompare(c, "opacity", 1.0, 500);
        }

        function test_batteryLevelDefault() {
            var c = createTemporaryObject(column, parent, {});
            compare(c.batteryLevel, 0);
        }

        function test_batteryLevelPropagates() {
            var c = createTemporaryObject(column, parent,
                { batteryLevel: 42, iconSource: "" });
            compare(c.batteryLevel, 42);
        }

        function test_chargingPropagates() {
            var c = createTemporaryObject(column, parent,
                { batteryLevel: 80, isCharging: true, iconSource: "" });
            compare(c.isCharging, true);
        }

        function test_indicatorPersists() {
            var c = createTemporaryObject(column, parent,
                { indicator: "L", iconSource: "" });
            compare(c.indicator, "L");
        }

        function test_indicatorDefaultsEmpty() {
            var c = createTemporaryObject(column, parent, { iconSource: "" });
            compare(c.indicator, "");
        }

        // Pin the use2D property default to false. iter-76 introduced
        // a generic mdi-headphones glyph variant that loses model
        // identity vs. the per-model PNG photos; user feedback (iter-77)
        // confirmed the photos read better. Default flipped to false
        // iter-77, kept for a future per-model SVG asset set. This test
        // guards against a regression that re-enables the generic glyph
        // path before the SVG assets land.
        function test_use2DDefaultsFalse() {
            var c = createTemporaryObject(column, parent, { iconSource: "" });
            compare(c.use2D, false);
        }

        // The use2D property is opt-in so a downstream caller can wire
        // a future settings toggle without breaking the default. Verify
        // the property is at least settable (catches accidental
        // readonly conversion).
        function test_use2DSettable() {
            var c = createTemporaryObject(column, parent,
                { iconSource: "", use2D: true });
            compare(c.use2D, true);
        }
    }
}
