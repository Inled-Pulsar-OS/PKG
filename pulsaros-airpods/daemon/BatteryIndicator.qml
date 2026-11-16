import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import linux 1.0

Rectangle {
    id: root

    property int batteryLevel: 50
    property bool isCharging: false
    property string indicator: ""
    readonly property color levelColor: {
        if (isCharging)
            return Theme.colCharging;

        if (batteryLevel <= 20)
            return Theme.colBatLow;

        if (batteryLevel <= 50)
            return Theme.colBatMid;

        return Theme.colBatHigh;
    }

    width: 86
    height: 44
    color: "transparent"

    ColumnLayout {
        anchors.fill: parent
        spacing: 6

        Item {
            id: batteryIcon

            Layout.preferredWidth: 34
            Layout.preferredHeight: 16
            Layout.alignment: Qt.AlignHCenter

            Rectangle {
                id: batteryBody

                width: parent.width - 2
                height: parent.height
                radius: 3
                color: "transparent"
                // Integer border for crisp 1-px edge; previous 1.4 fell
                // between rasters and looked smeared at scale 1.0.
                border.width: Theme.borderWidth
                border.color: Theme.colDim

                Rectangle {
                    id: batteryFill

                    // Pulse opacity for two distinct urgent states:
                    //   - charging: indicates vitality / inbound power
                    //   - critical low (<=10% and not charging): warning
                    // Both pulse the same 1.0 ↔ 0.55 ramp so the visual
                    // language stays consistent; color (levelColor) is
                    // already different between the two states.
                    readonly property bool shouldPulse: root.isCharging || (root.batteryLevel <= 10 && !root.isCharging)

                    width: Math.max(2, (batteryBody.width - 4) * (root.batteryLevel / 100))
                    height: batteryBody.height - 4
                    anchors.left: parent.left
                    anchors.leftMargin: 2
                    anchors.verticalCenter: parent.verticalCenter
                    radius: 2
                    color: root.levelColor

                    SequentialAnimation {
                        running: batteryFill.shouldPulse
                        loops: Animation.Infinite
                        alwaysRunToEnd: true

                        NumberAnimation {
                            target: batteryFill
                            property: "opacity"
                            to: 0.55
                            duration: 2200
                        }

                        NumberAnimation {
                            target: batteryFill
                            property: "opacity"
                            to: 1
                            duration: 2200
                        }
                    }

                    // alwaysRunToEnd lets the *current* half-cycle complete
                    // when the pulse condition flips off — but if it landed
                    // on 0.55 the fill stays dim. Snap to fully opaque when
                    // we leave the pulsing state.
                    Connections {
                        function onShouldPulseChanged() {
                            if (!batteryFill.shouldPulse)
                                batteryFill.opacity = 1;
                        }

                        target: batteryFill
                    }

                    Behavior on width {
                        NumberAnimation {
                            duration: 300
                            easing.type: Easing.OutCubic
                        }
                    }

                    Behavior on color {
                        ColorAnimation {
                            duration: 200
                        }
                    }
                }
            }

            Rectangle {
                width: 2
                height: 8
                radius: 1
                color: Theme.colDim
                anchors.left: batteryBody.right
                anchors.verticalCenter: batteryBody.verticalCenter
            }

            Text {
                visible: root.isCharging
                anchors.centerIn: batteryBody
                text: "⚡"
                color: Theme.colFg
                font.pixelSize: 11
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 5

            Rectangle {
                id: indicatorBackground

                visible: root.indicator !== ""
                Layout.preferredWidth: 16
                Layout.preferredHeight: Layout.preferredWidth
                // Geometric circle: radius = width/2. tileRadius (=8)
                // happens to equal half-of-16 today but bumping the
                // indicator size would silently drift to a rounded square.
                radius: Layout.preferredWidth / 2
                color: Theme.colToggleOff
                border.color: Theme.colBorder
                border.width: 1

                Text {
                    anchors.centerIn: parent
                    text: root.indicator
                    color: Theme.colFg
                    font.pixelSize: 10
                    font.family: Theme.fontFamily
                    font.weight: Font.DemiBold
                }
            }

            Text {
                text: root.batteryLevel + "%"
                color: Theme.colFg
                font.pixelSize: 12
                font.family: Theme.fontFamily
                // Tabular numerals so percentages keep a fixed column
                // width as the value ticks. SF Pro Text ships tnum.
                font.features: {
                    "tnum": 1
                }
            }
        }
    }
}
