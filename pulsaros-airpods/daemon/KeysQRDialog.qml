import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Controls.Basic 2.15
import QtQuick.Layouts 1.15
import QtQuick.Window 2.15
import linux 1.0

Window {
    id: root

    property string irk: ""
    property string encKey: ""

    title: "Magic Cloud Keys QR Code"
    flags: Qt.Dialog
    modality: Qt.WindowModal
    color: Theme.colBg
    width: Math.min(Screen.width * 0.8, 360)
    height: Math.min(Screen.height * 0.7, 420)

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 16

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: width
            radius: Theme.radius
            // QR codes need white background regardless of theme — keep
            // them as the canonical Theme.colFg (white) so scanners read
            // them reliably; error text below uses Theme.colBg (black)
            // for the same paper-and-ink semantics.
            color: Theme.colFg
            border.color: Theme.colBorder
            border.width: Theme.borderWidth

            Image {
                id: qrCodeImage

                anchors.centerIn: parent
                width: Math.min(parent.width * 0.88, parent.height * 0.88)
                height: width
                fillMode: Image.PreserveAspectFit
                smooth: false
                // Skip the request entirely when keys aren't loaded yet so
                // the image provider doesn't waste CPU rendering a QR for
                // ";".
                source: (root.encKey.length > 0 && root.irk.length > 0) ? ("image://qrcode/" + root.encKey + ";" + root.irk) : ""

                BusyIndicator {
                    anchors.centerIn: parent
                    running: qrCodeImage.status === Image.Loading
                }

                Text {
                    anchors.centerIn: parent
                    visible: qrCodeImage.status === Image.Error || qrCodeImage.source.toString() === ""
                    text: qrCodeImage.source.toString() === "" ? qsTr("Waiting for keys…") : qsTr("Failed to generate QR code")
                    color: Theme.colBg
                    font.family: Theme.fontFamily
                    font.pixelSize: 12
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Scan this QR code to transfer the Magic Cloud Keys to another device.")
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            color: Theme.colDim
            font.family: Theme.fontFamily
            font.pixelSize: 12
        }
    }
}
