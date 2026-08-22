import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

ColumnLayout {
    id: clockRoot
    spacing: 2

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

    Label {
        text: Qt.formatDateTime(clockRoot.currentTime, "dddd, MMMM d")
        color: "white"
        opacity: 0.5
        style: softwareRendering ? Text.Outline : Text.Normal
        styleColor: softwareRendering ? ColorScope.backgroundColor : "transparent"
        font.pointSize: 20
        font.weight: Font.DemiBold
        font.capitalization: Font.Capitalize
        Layout.alignment: Qt.AlignHCenter
        font.family: fontbold.name
    }

    Label {
        text: Qt.formatDateTime(clockRoot.currentTime, "h:mm")
        color: "white"
        opacity: 0.5
        style: softwareRendering ? Text.Outline : Text.Normal
        styleColor: softwareRendering ? ColorScope.backgroundColor : "transparent"
        font.pointSize: 100
        font.bold: true
        Layout.alignment: Qt.AlignHCenter
        font.family: fontbold.name
    }
}
