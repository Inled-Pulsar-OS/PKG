import QtQuick

Item {
    id: spinnerRoot
    implicitWidth: 32
    implicitHeight: 32
    property bool running: true
    property color color: "#ffffff"

    Item {
        id: spinnerContainer
        anchors.centerIn: parent
        width: Math.min(parent.width, parent.height)
        height: width

        Repeater {
            model: 12
            Item {
                id: bladeHolder
                anchors.fill: parent
                rotation: index * 30
                transformOrigin: Item.Center

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: Math.max(1, parent.height * 0.04)
                    width: Math.max(2, parent.width * 0.08)
                    height: Math.max(5, parent.height * 0.25)
                    radius: width / 2
                    color: spinnerRoot.color
                    opacity: 0.15 + 0.85 * (index / 11.0)
                }
            }
        }

        RotationAnimation on rotation {
            from: 0
            to: 360
            duration: 900
            loops: Animation.Infinite
            running: spinnerRoot.running && spinnerRoot.visible
        }
    }
}
