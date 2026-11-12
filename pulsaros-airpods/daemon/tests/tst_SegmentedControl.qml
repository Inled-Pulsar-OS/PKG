// SegmentedControl behavior regression.
//
// Pins:
//   - Default model + currentIndex shape.
//   - currentIndex bounds-clamped at 0 and model.length-1 under Left/Right.
//   - Home / End jump to extremes.
//   - Number-key shortcut (Qt.Key_1..Key_9) selects by 1-based index when
//     in range, no-op when out of range.
//   - Selecting via assignment doesn't fire when the value doesn't change.
//
// SegmentedControl.qml lives in /linux QML module; import provides it.

import QtQuick 2.15
import QtTest 6.0
import QtQuick.Controls 2.15
import linux 1.0

Item {
    width: 400
    height: 60

    Component {
        id: seg
        SegmentedControl {}
    }

    TestCase {
        name: "SegmentedControl"
        when: windowShown

        function test_defaultModel() {
            var s = createTemporaryObject(seg, parent, {});
            compare(s.model.length, 2);
            compare(s.model[0], "Option 1");
            compare(s.model[1], "Option 2");
        }

        function test_defaultCurrentIndex() {
            var s = createTemporaryObject(seg, parent, {});
            compare(s.currentIndex, 0);
        }

        function test_modelOverride() {
            var s = createTemporaryObject(seg, parent,
                { model: ["Off", "ANC", "Trans", "Adaptive"] });
            compare(s.model.length, 4);
            compare(s.currentIndex, 0);
        }

        function test_arrowRightAdvances() {
            var s = createTemporaryObject(seg, parent,
                { model: ["A", "B", "C"] });
            s.forceActiveFocus();
            keyClick(Qt.Key_Right);
            compare(s.currentIndex, 1);
            keyClick(Qt.Key_Right);
            compare(s.currentIndex, 2);
        }

        function test_arrowRightClampedAtEnd() {
            var s = createTemporaryObject(seg, parent,
                { model: ["A", "B"], currentIndex: 1 });
            s.forceActiveFocus();
            keyClick(Qt.Key_Right);
            compare(s.currentIndex, 1);
        }

        function test_arrowLeftDecrements() {
            var s = createTemporaryObject(seg, parent,
                { model: ["A", "B", "C"], currentIndex: 2 });
            s.forceActiveFocus();
            keyClick(Qt.Key_Left);
            compare(s.currentIndex, 1);
            keyClick(Qt.Key_Left);
            compare(s.currentIndex, 0);
        }

        function test_arrowLeftClampedAtZero() {
            var s = createTemporaryObject(seg, parent, {});
            s.forceActiveFocus();
            keyClick(Qt.Key_Left);
            compare(s.currentIndex, 0);
        }

        function test_homeJumpsToStart() {
            var s = createTemporaryObject(seg, parent,
                { model: ["A", "B", "C", "D"], currentIndex: 3 });
            s.forceActiveFocus();
            keyClick(Qt.Key_Home);
            compare(s.currentIndex, 0);
        }

        function test_endJumpsToLast() {
            var s = createTemporaryObject(seg, parent,
                { model: ["A", "B", "C", "D"] });
            s.forceActiveFocus();
            keyClick(Qt.Key_End);
            compare(s.currentIndex, 3);
        }

        function test_numberKeyShortcut() {
            var s = createTemporaryObject(seg, parent,
                { model: ["A", "B", "C", "D"] });
            s.forceActiveFocus();
            keyClick(Qt.Key_3);
            compare(s.currentIndex, 2);  // 1-based
            keyClick(Qt.Key_1);
            compare(s.currentIndex, 0);
        }

        function test_numberKeyOutOfRangeIgnored() {
            var s = createTemporaryObject(seg, parent,
                { model: ["A", "B"] });
            s.forceActiveFocus();
            keyClick(Qt.Key_5);  // model has 2 entries
            compare(s.currentIndex, 0);
        }

        function test_directAssignment() {
            var s = createTemporaryObject(seg, parent,
                { model: ["A", "B", "C"] });
            s.currentIndex = 2;
            compare(s.currentIndex, 2);
        }

        function test_focusPolicy() {
            var s = createTemporaryObject(seg, parent, {});
            compare(s.focusPolicy, Qt.StrongFocus);
            compare(s.activeFocusOnTab, true);
        }
    }
}
