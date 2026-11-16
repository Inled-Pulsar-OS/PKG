pragma ComponentBehavior: Bound

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Controls.Basic 2.15
import linux 1.0

Control {
    id: root

    property var model: ["Option 1", "Option 2"]
    property int currentIndex: 0

    padding: 4
    implicitHeight: 38

    focusPolicy: Qt.StrongFocus
    activeFocusOnTab: true

    background: Rectangle {
        radius: Theme.tileRadius
        color: Theme.colToggleOff
        border.width: 1
        border.color: root.activeFocus ? Theme.colAccent : Theme.colBorder
    }

    contentItem: Row {
        spacing: 0

        Repeater {
            model: root.model

            delegate: Item {
                id: cell
                required property int index
                required property string modelData
                readonly property bool selected: root.currentIndex === cell.index
                width: (root.availableWidth) / root.model.length
                height: root.availableHeight

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 0
                    radius: 6
                    color: cell.selected ? Theme.colToggleOn : (cellHover.containsMouse ? Theme.colBorder : "transparent")
                    // When the segmented control has keyboard focus,
                    // outline the currently-selected cell with the accent
                    // colour so arrow-key users see which one Enter would
                    // re-activate. Non-selected cells stay borderless.
                    border.color: Theme.colAccent
                    border.width: (root.activeFocus && cell.selected) ? 1 : 0

                    Behavior on color {
                        ColorAnimation {
                            duration: 180
                            easing.type: Easing.OutCubic
                        }
                    }

                    Text {
                        anchors.centerIn: parent
                        text: cell.modelData
                        font.family: Theme.fontFamily
                        font.pixelSize: 12
                        font.weight: cell.selected ? Font.DemiBold : Font.Normal
                        color: cell.selected ? Theme.colBg : Theme.colFg
                        elide: Text.ElideRight
                        Behavior on color {
                            ColorAnimation {
                                duration: 180
                            }
                        }
                    }

                    MouseArea {
                        id: cellHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (!cell.selected)
                                root.currentIndex = cell.index;
                        }
                    }
                }
            }
        }
    }

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Left) {
            if (root.currentIndex > 0) {
                root.currentIndex--;
                event.accepted = true;
            }
        } else if (event.key === Qt.Key_Right) {
            if (root.currentIndex < root.model.length - 1) {
                root.currentIndex++;
                event.accepted = true;
            }
        } else if (event.key === Qt.Key_Home) {
            root.currentIndex = 0;
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            root.currentIndex = root.model.length - 1;
            event.accepted = true;
        } else if (event.key >= Qt.Key_1 && event.key <= Qt.Key_9) {
            const index = event.key - Qt.Key_1;
            if (index < root.model.length) {
                root.currentIndex = index;
                event.accepted = true;
            }
        }
    }
}
