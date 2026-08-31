import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Qt5Compat.GraphicalEffects

Item {
    id: clockRoot
    implicitWidth: clockLayout.implicitWidth
    implicitHeight: clockLayout.implicitHeight

    FontLoader {
        id: fontbold
        source: "../fonts/SFUIText-Semibold.otf"
    }

    readonly property bool softwareRendering: GraphicsInfo.api === GraphicsInfo.Software
    property date currentTime: new Date()

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            clockRoot.currentTime = new Date()
        }
    }

    ColumnLayout {
        id: clockLayout
        anchors.centerIn: parent
        spacing: 4

        Label {
            text: Qt.formatDateTime(clockRoot.currentTime, "dddd, MMMM d")
            color: "#ffffff"
            opacity: 0.9
            style: softwareRendering ? Text.Outline : Text.Normal
            styleColor: softwareRendering ? ColorScope.backgroundColor : "transparent"
            font.pointSize: 16
            font.weight: Font.DemiBold
            font.capitalization: Font.Capitalize
            Layout.alignment: Qt.AlignHCenter
            font.family: fontbold.name

            layer.enabled: true
            layer.effect: DropShadow {
                horizontalOffset: 0
                verticalOffset: 2
                radius: 8
                samples: 16
                color: "#60000000"
            }
        }

        Label {
            text: Qt.formatDateTime(clockRoot.currentTime, "H:mm")
            color: "#ffffff"
            style: softwareRendering ? Text.Outline : Text.Normal
            styleColor: softwareRendering ? ColorScope.backgroundColor : "transparent"
            font.pointSize: 84
            font.bold: true
            Layout.alignment: Qt.AlignHCenter
            font.family: fontbold.name

            layer.enabled: true
            layer.effect: DropShadow {
                horizontalOffset: 0
                verticalOffset: 4
                radius: 18
                samples: 24
                color: "#70000000"
            }
        }
    }
}
