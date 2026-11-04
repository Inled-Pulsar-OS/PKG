pragma ComponentBehavior: Bound

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Controls.Basic 2.15
import QtQuick.Layouts 1.15
import linux 1.0

ApplicationWindow {
    id: mainWindow
    visible: !airPodsTrayApp.hideOnStart
    // Width matches Quickshell CommandCenter.qml contentPanel (380px) so
    // side-by-side the two surfaces read as one design family.
    width: 380
    height: 560
    minimumWidth: 360
    minimumHeight: 480
    title: "OpenPods"
    objectName: "mainWindowObject"
    color: Theme.colBg

    onClosing: mainWindow.visible = false

    function reopen(pageToLoad) {
        if (pageToLoad == "settings") {
            if (stackView.depth == 1)
                stackView.push(settingsPage);
        } else {
            if (stackView.depth > 1)
                stackView.pop();
        }
        if (!mainWindow.visible)
            mainWindow.visible = true;
        raise();
        requestActivate();
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.BackButton | Qt.ForwardButton
        onClicked: mouse => {
            if (mouse.button === Qt.BackButton && stackView.depth > 1)
                stackView.pop();
        }
    }

    StackView {
        id: stackView
        anchors.fill: parent
        initialItem: mainPage
    }

    component CardRect: Rectangle {
        radius: Theme.radius
        color: Theme.colSurface
        border.color: Theme.colBorder
        border.width: Theme.borderWidth
    }

    component SectionLabel: Text {
        font.family: Theme.fontFamily
        font.pixelSize: 11
        font.weight: Font.Medium
        font.letterSpacing: 0.6
        color: Theme.colDim
    }

    component IconButton: Rectangle {
        id: ib
        property string glyph: ""
        property int glyphSize: 16
        // Human-readable name for screen readers — glyph alone (a single
        // Nerd Font codepoint) is meaningless to AT-SPI.
        property string name: ""
        signal clicked
        Accessible.role: Accessible.Button
        Accessible.name: ib.name
        Accessible.onPressAction: ib.clicked()
        activeFocusOnTab: true
        Keys.onReturnPressed: ib.clicked()
        Keys.onSpacePressed: ib.clicked()
        width: 32
        height: 32
        radius: 16
        // Three-state visual: pressed > hover > rest. Pressed darkens via
        // Theme.colBorder (#2a2a2a) — distinct enough from the hover tint
        // (Theme.colToggleOff #1a1a1a) that the click feels tactile.
        color: hover.pressed ? Theme.colBorder : hover.containsMouse ? Theme.colToggleOff : "transparent"
        // Focus ring: 1px Theme.colAccent border replaces the default
        // Theme.colBorder so keyboard-tabbed buttons show their state.
        border.color: ib.activeFocus ? Theme.colAccent : Theme.colBorder
        border.width: 1
        Behavior on color {
            ColorAnimation {
                duration: 120
                easing.type: Easing.OutCubic
            }
        }
        Text {
            anchors.centerIn: parent
            text: ib.glyph
            font.family: Theme.iconFont
            font.pixelSize: ib.glyphSize
            color: Theme.colFg
        }
        MouseArea {
            id: hover
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: ib.clicked()
        }
    }

    component QSSwitch: Rectangle {
        id: sw
        property bool checked: false
        property string name: ""
        signal toggled(bool value)
        Accessible.role: Accessible.CheckBox
        Accessible.name: sw.name
        Accessible.checkable: true
        Accessible.checked: sw.checked
        Accessible.onToggleAction: sw.toggled(!sw.checked)
        width: 38
        height: 22
        radius: 11
        color: sw.checked ? Theme.colToggleOn : Theme.colToggleOff
        border.color: Theme.colBorder
        border.width: 1
        // Match Quickshell AudioControls.qml: 180ms OutCubic for color
        // transitions, 120ms OutCubic for movement.
        Behavior on color {
            ColorAnimation {
                duration: 180
                easing.type: Easing.OutCubic
            }
        }

        Rectangle {
            width: 16
            height: 16
            radius: 8
            color: sw.checked ? Theme.colBg : Theme.colFg
            anchors.verticalCenter: parent.verticalCenter
            x: sw.checked ? parent.width - width - 3 : 3
            Behavior on x {
                NumberAnimation {
                    duration: 120
                    easing.type: Easing.OutCubic
                }
            }
            Behavior on color {
                ColorAnimation {
                    duration: 180
                    easing.type: Easing.OutCubic
                }
            }
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: sw.toggled(!sw.checked)
        }
    }

    component RowSwitch: Item {
        id: rs
        property string label: ""
        property string sublabel: ""
        property bool checked: false
        signal toggled(bool value)
        // Composite-row a11y: row reads as a checkbox; the inner QSSwitch
        // is purely visual once the row owns the role.
        Accessible.role: Accessible.CheckBox
        Accessible.name: rs.label
        Accessible.description: rs.sublabel
        Accessible.checkable: true
        Accessible.checked: rs.checked
        Accessible.onToggleAction: rs.toggled(!rs.checked)
        implicitHeight: Math.max(44, rsCol.implicitHeight + 12)
        // Inside a ColumnLayout, raw `width: parent.width` fights the
        // layout engine and the row collapses. Layout.fillWidth lets
        // ColumnLayout drive the size correctly. implicitWidth keeps the
        // component usable outside a Layout too.
        implicitWidth: parent ? parent.width : 0
        Layout.fillWidth: true

        RowLayout {
            anchors.fill: parent
            spacing: 12

            ColumnLayout {
                id: rsCol
                Layout.fillWidth: true
                spacing: 2
                Text {
                    text: rs.label
                    font.family: Theme.fontFamily
                    font.pixelSize: 13
                    color: Theme.colFg
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                }
                Text {
                    text: rs.sublabel
                    visible: rs.sublabel !== ""
                    font.family: Theme.fontFamily
                    font.pixelSize: 11
                    color: Theme.colDim
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                }
            }

            QSSwitch {
                checked: rs.checked
                onToggled: v => rs.toggled(v)
            }
        }
    }

    component QSButton: Rectangle {
        id: btn
        property string label: ""
        property bool primary: false
        signal clicked
        Accessible.role: Accessible.Button
        Accessible.name: btn.label
        Accessible.onPressAction: btn.clicked()
        activeFocusOnTab: true
        Keys.onReturnPressed: btn.clicked()
        Keys.onSpacePressed: btn.clicked()
        implicitHeight: 32
        implicitWidth: btnLbl.implicitWidth + 24
        radius: Theme.tileRadius
        color: btn.primary ? (mouseBtn.containsMouse ? Qt.darker(Theme.colToggleOn, 1.05) : Theme.colToggleOn) : (mouseBtn.containsMouse ? Theme.colBorder : Theme.colToggleOff)
        border.color: btn.activeFocus ? Theme.colAccent : Theme.colBorder
        border.width: 1
        Behavior on color {
            ColorAnimation {
                duration: 180
                easing.type: Easing.OutCubic
            }
        }
        Text {
            id: btnLbl
            anchors.centerIn: parent
            text: btn.label
            font.family: Theme.fontFamily
            font.pixelSize: 12
            font.weight: btn.primary ? Font.DemiBold : Font.Normal
            color: btn.primary ? Theme.colBg : Theme.colFg
        }
        MouseArea {
            id: mouseBtn
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: btn.clicked()
        }
    }

    component QSTextField: Rectangle {
        id: tf
        property alias text: input.text
        property string placeholder: ""
        implicitHeight: 32
        implicitWidth: 160
        radius: Theme.tileRadius
        color: Theme.colToggleOff
        border.color: input.activeFocus ? Theme.colAccent : Theme.colBorder
        border.width: 1
        TextInput {
            id: input
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.colFg
            font.family: Theme.fontFamily
            font.pixelSize: 12
            clip: true
            selectByMouse: true
            selectionColor: Theme.colAccent
            selectedTextColor: Theme.colBg
        }
        Text {
            anchors.left: parent.left
            anchors.leftMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            text: tf.placeholder
            visible: input.text === ""
            color: Theme.colDim
            font.family: Theme.fontFamily
            font.pixelSize: 12
        }
    }

    Component {
        id: mainPage
        Item {
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 16

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    Text {
                        text: airPodsTrayApp.airpodsConnected && airPodsTrayApp.deviceInfo.deviceName ? airPodsTrayApp.deviceInfo.deviceName : "OpenPods"
                        font.family: Theme.fontFamily
                        font.pixelSize: 16
                        font.weight: Font.DemiBold
                        color: Theme.colFg
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        // Geometric pill: radius = implicitWidth/2 self-documents
                        // the circle relationship. Use implicit{Width,Height}
                        // inside RowLayout — raw width/height fights the
                        // layout engine (qmllint Quick.layout-positioning).
                        implicitWidth: 8
                        implicitHeight: implicitWidth
                        radius: implicitWidth / 2
                        color: airPodsTrayApp.airpodsConnected ? Theme.colOnline : Theme.colUrgent
                        Layout.alignment: Qt.AlignVCenter
                    }

                    Text {
                        text: airPodsTrayApp.airpodsConnected ? qsTr("Connected") : qsTr("Disconnected")
                        font.family: Theme.fontFamily
                        font.pixelSize: 11
                        color: Theme.colDim
                        Layout.alignment: Qt.AlignVCenter
                    }

                    IconButton {
                        glyph: "\u{F0493}"
                        glyphSize: 14
                        name: qsTr("Settings")
                        Layout.alignment: Qt.AlignVCenter
                        onClicked: stackView.push(settingsPage)
                    }
                }

                CardRect {
                    Layout.fillWidth: true
                    // Content-driven: PodColumn implicitHeight + 16px
                    // vertical padding top and bottom. Tracks any change
                    // to the pod-image height instead of drifting from
                    // a hardcoded 156.
                    Layout.preferredHeight: podsRow.implicitHeight + 32
                    // Fade in/out on connect/disconnect rather than a hard
                    // visibility flip. `visible` stays true while opacity
                    // animates so the Behavior runs both directions; we
                    // skip layout when fully transparent.
                    opacity: airPodsTrayApp.airpodsConnected ? 1 : 0
                    visible: opacity > 0
                    Behavior on opacity {
                        NumberAnimation {
                            duration: 180
                            easing.type: Easing.OutCubic
                        }
                    }

                    Row {
                        id: podsRow
                        anchors.centerIn: parent
                        spacing: 14

                        PodColumn {
                            visible: airPodsTrayApp.deviceInfo.battery.leftPodAvailable
                            inEar: airPodsTrayApp.deviceInfo.leftPodInEar
                            iconSource: "qrc:/icons/assets/" + airPodsTrayApp.deviceInfo.podIcon
                            batteryLevel: airPodsTrayApp.deviceInfo.battery.leftPodLevel
                            isCharging: airPodsTrayApp.deviceInfo.battery.leftPodCharging
                            indicator: "L"
                        }
                        PodColumn {
                            visible: airPodsTrayApp.deviceInfo.battery.rightPodAvailable
                            inEar: airPodsTrayApp.deviceInfo.rightPodInEar
                            iconSource: "qrc:/icons/assets/" + airPodsTrayApp.deviceInfo.podIcon
                            batteryLevel: airPodsTrayApp.deviceInfo.battery.rightPodLevel
                            isCharging: airPodsTrayApp.deviceInfo.battery.rightPodCharging
                            indicator: "R"
                        }
                        PodColumn {
                            visible: airPodsTrayApp.deviceInfo.battery.caseAvailable
                            inEar: true
                            iconSource: "qrc:/icons/assets/" + airPodsTrayApp.deviceInfo.caseIcon
                            batteryLevel: airPodsTrayApp.deviceInfo.battery.caseLevel
                            isCharging: airPodsTrayApp.deviceInfo.battery.caseCharging
                        }
                        PodColumn {
                            visible: airPodsTrayApp.deviceInfo.battery.headsetAvailable
                            inEar: true
                            iconSource: "qrc:/icons/assets/" + airPodsTrayApp.deviceInfo.podIcon
                            batteryLevel: airPodsTrayApp.deviceInfo.battery.headsetLevel
                            isCharging: airPodsTrayApp.deviceInfo.battery.headsetCharging
                        }
                    }

                    Text {
                        anchors.centerIn: parent
                        visible: !airPodsTrayApp.deviceInfo.battery.leftPodAvailable && !airPodsTrayApp.deviceInfo.battery.rightPodAvailable && !airPodsTrayApp.deviceInfo.battery.caseAvailable && !airPodsTrayApp.deviceInfo.battery.headsetAvailable
                        text: qsTr("Waiting for battery info…")
                        font.family: Theme.fontFamily
                        font.pixelSize: 12
                        color: Theme.colDim
                    }
                }

                CardRect {
                    Layout.fillWidth: true
                    // Content-driven: empty-state copy + 32px vertical
                    // padding. If the strings shrink or grow, the card
                    // height stays proportional.
                    Layout.preferredHeight: emptyStateCol.implicitHeight + 48
                    // Inverse fade: shows while disconnected, hides on
                    // connect. Same 180ms OutCubic so the two cards
                    // crossfade through each other on the state change.
                    opacity: airPodsTrayApp.airpodsConnected ? 0 : 1
                    visible: opacity > 0
                    Behavior on opacity {
                        NumberAnimation {
                            duration: 180
                            easing.type: Easing.OutCubic
                        }
                    }
                    Column {
                        id: emptyStateCol
                        anchors.centerIn: parent
                        spacing: 6
                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: qsTr("No AirPods connected")
                            font.family: Theme.fontFamily
                            font.pixelSize: 13
                            color: Theme.colFg
                        }
                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: qsTr("Pair via bluetoothctl, then open the case lid.")
                            font.family: Theme.fontFamily
                            font.pixelSize: 11
                            color: Theme.colDim
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    visible: airPodsTrayApp.airpodsConnected

                    SectionLabel {
                        text: qsTr("NOISE CONTROL")
                    }

                    SegmentedControl {
                        Layout.fillWidth: true
                        model: [qsTr("Off"), qsTr("ANC"), qsTr("Transparency"), qsTr("Adaptive")]
                        currentIndex: airPodsTrayApp.deviceInfo.noiseControlMode
                        onCurrentIndexChanged: airPodsTrayApp.setNoiseControlModeInt(currentIndex)
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        visible: airPodsTrayApp.deviceInfo.adaptiveModeActive

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: qsTr("Adaptive level")
                                font.family: Theme.fontFamily
                                font.pixelSize: 11
                                color: Theme.colDim
                                Layout.fillWidth: true
                            }
                            Text {
                                text: Math.round(adaptiveSlider.value) + "%"
                                font.family: Theme.fontFamily
                                font.pixelSize: 11
                                color: Theme.colFg
                                // Tabular figures so the label doesn't
                                // shimmy as the slider drags across the
                                // 0–100 range (matches iter-6 pattern).
                                font.features: {
                                    "tnum": 1
                                }
                            }
                        }

                        Slider {
                            id: adaptiveSlider
                            Layout.fillWidth: true
                            from: 0
                            to: 100
                            stepSize: 1
                            value: airPodsTrayApp.deviceInfo.adaptiveNoiseLevel
                            // 32 matches the height of every other input
                            // row (QSButton, QSTextField). Track is still
                            // 4px — extra height is touch target, which
                            // Apple HIG recommends ≥32 even for thin
                            // sliders.
                            implicitHeight: 32

                            background: Rectangle {
                                x: adaptiveSlider.leftPadding
                                y: adaptiveSlider.topPadding + adaptiveSlider.availableHeight / 2 - 2
                                width: adaptiveSlider.availableWidth
                                height: 4
                                radius: 2
                                color: Theme.colToggleOff
                                Rectangle {
                                    width: adaptiveSlider.visualPosition * parent.width
                                    height: parent.height
                                    color: Theme.colAccent
                                    radius: 2
                                }
                            }
                            handle: Rectangle {
                                x: adaptiveSlider.leftPadding + adaptiveSlider.visualPosition * (adaptiveSlider.availableWidth - width)
                                y: adaptiveSlider.topPadding + adaptiveSlider.availableHeight / 2 - height / 2
                                width: 14
                                height: 14
                                radius: 7
                                color: Theme.colFg
                                border.color: Theme.colBorder
                                border.width: 1
                            }

                            Timer {
                                id: debounceTimer
                                interval: 500
                                onTriggered: if (!adaptiveSlider.pressed)
                                    airPodsTrayApp.setAdaptiveNoiseLevel(adaptiveSlider.value)
                            }
                            onPressedChanged: if (!pressed)
                                airPodsTrayApp.setAdaptiveNoiseLevel(value)
                            onValueChanged: if (pressed)
                                debounceTimer.restart()
                        }
                    }
                }

                CardRect {
                    Layout.fillWidth: true
                    Layout.preferredHeight: togglesCol.implicitHeight + 24
                    visible: airPodsTrayApp.airpodsConnected

                    ColumnLayout {
                        id: togglesCol
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 4

                        RowSwitch {
                            label: qsTr("Conversational Awareness")
                            checked: airPodsTrayApp.deviceInfo.conversationalAwareness
                            onToggled: v => airPodsTrayApp.setConversationalAwareness(v)
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 1
                            color: Theme.colBorder
                        }
                        RowSwitch {
                            label: qsTr("Hearing Aid")
                            checked: airPodsTrayApp.deviceInfo.hearingAidEnabled
                            onToggled: v => airPodsTrayApp.setHearingAidEnabled(v)
                        }
                    }
                }

                Item {
                    Layout.fillHeight: true
                }
            }
        }
    }

    Component {
        id: settingsPage
        Item {
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 16

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    IconButton {
                        glyph: "\u{F0141}"
                        glyphSize: 14
                        name: qsTr("Back")
                        Layout.alignment: Qt.AlignVCenter
                        onClicked: stackView.pop()
                    }
                    Text {
                        text: qsTr("Settings")
                        font.family: Theme.fontFamily
                        font.pixelSize: 16
                        font.weight: Font.DemiBold
                        color: Theme.colFg
                        Layout.fillWidth: true
                    }
                }

                Flickable {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentHeight: settingsCol.implicitHeight
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds

                    ColumnLayout {
                        id: settingsCol
                        width: parent.width
                        spacing: 16

                        CardRect {
                            Layout.fillWidth: true
                            Layout.preferredHeight: edCol.implicitHeight + 24

                            ColumnLayout {
                                id: edCol
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 8

                                SectionLabel {
                                    text: qsTr("PAUSE WHEN REMOVING")
                                }

                                SegmentedControl {
                                    Layout.fillWidth: true
                                    model: [qsTr("One"), qsTr("Both"), qsTr("Never")]
                                    currentIndex: airPodsTrayApp.earDetectionBehavior
                                    onCurrentIndexChanged: airPodsTrayApp.earDetectionBehavior = currentIndex
                                }
                            }
                        }

                        CardRect {
                            Layout.fillWidth: true
                            Layout.preferredHeight: switchCol.implicitHeight + 24

                            ColumnLayout {
                                id: switchCol
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 4

                                RowSwitch {
                                    label: qsTr("Cross-device with Android")
                                    sublabel: qsTr("Hand-off via companion phone")
                                    checked: airPodsTrayApp.crossDeviceEnabled
                                    onToggled: v => airPodsTrayApp.setCrossDeviceEnabled(v)
                                }
                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 1
                                    color: Theme.colBorder
                                }
                                RowSwitch {
                                    label: qsTr("Auto-start on login")
                                    checked: airPodsTrayApp.autoStartManager.autoStartEnabled
                                    onToggled: v => airPodsTrayApp.autoStartManager.autoStartEnabled = v
                                }
                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 1
                                    color: Theme.colBorder
                                }
                                RowSwitch {
                                    label: qsTr("System notifications")
                                    checked: airPodsTrayApp.notificationsEnabled
                                    onToggled: v => airPodsTrayApp.notificationsEnabled = v
                                }
                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 1
                                    color: Theme.colBorder
                                    visible: airPodsTrayApp.airpodsConnected
                                }
                                RowSwitch {
                                    visible: airPodsTrayApp.airpodsConnected
                                    label: qsTr("One-bud ANC")
                                    sublabel: qsTr("More noise reduction, more battery")
                                    checked: airPodsTrayApp.deviceInfo.oneBudANCMode
                                    // Route through AirPodsTrayApp::setOneBudANCMode
                                    // so the BT packet actually goes out — direct
                                    // assignment to deviceInfo.oneBudANCMode only
                                    // flipped the local UI flag.
                                    onToggled: v => airPodsTrayApp.setOneBudANCMode(v)
                                }
                            }
                        }

                        CardRect {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 56

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 12

                                Text {
                                    text: qsTr("Bluetooth retry attempts")
                                    font.family: Theme.fontFamily
                                    font.pixelSize: 13
                                    color: Theme.colFg
                                    Layout.fillWidth: true
                                }

                                QSButton {
                                    label: "−"
                                    onClicked: if (airPodsTrayApp.retryAttempts > 1)
                                        airPodsTrayApp.retryAttempts--
                                }
                                Text {
                                    text: airPodsTrayApp.retryAttempts
                                    font.family: Theme.fontFamily
                                    font.pixelSize: 13
                                    color: Theme.colFg
                                    horizontalAlignment: Text.AlignHCenter
                                    Layout.preferredWidth: 24
                                    // Tabular numerals so the +/- buttons
                                    // don't shimmy as the count moves
                                    // between single and double digits.
                                    font.features: {
                                        "tnum": 1
                                    }
                                }
                                QSButton {
                                    label: "+"
                                    onClicked: if (airPodsTrayApp.retryAttempts < 10)
                                        airPodsTrayApp.retryAttempts++
                                }
                            }
                        }

                        CardRect {
                            Layout.fillWidth: true
                            Layout.preferredHeight: renameCol.implicitHeight + 24
                            visible: airPodsTrayApp.airpodsConnected

                            ColumnLayout {
                                id: renameCol
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 8

                                SectionLabel {
                                    text: qsTr("RENAME AIRPODS")
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    QSTextField {
                                        id: newNameField
                                        Layout.fillWidth: true
                                        placeholder: airPodsTrayApp.deviceInfo.deviceName || qsTr("New name")
                                    }
                                    QSButton {
                                        label: qsTr("Rename")
                                        primary: true
                                        onClicked: airPodsTrayApp.renameAirPods(newNameField.text)
                                    }
                                }
                            }
                        }

                        CardRect {
                            Layout.fillWidth: true
                            Layout.preferredHeight: phoneCol.implicitHeight + 24
                            visible: airPodsTrayApp.airpodsConnected

                            ColumnLayout {
                                id: phoneCol
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 8

                                SectionLabel {
                                    text: qsTr("PHONE MAC")
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    QSTextField {
                                        id: newPhoneMacField
                                        Layout.fillWidth: true
                                        placeholder: (PHONE_MAC_ADDRESS !== "" ? PHONE_MAC_ADDRESS : "00:00:00:00:00:00")
                                    }
                                    QSButton {
                                        label: qsTr("Set")
                                        primary: true
                                        onClicked: airPodsTrayApp.setPhoneMac(newPhoneMacField.text)
                                    }
                                }
                            }
                        }

                        QSButton {
                            Layout.fillWidth: true
                            implicitHeight: 36
                            label: qsTr("Show Magic Cloud Keys QR")
                            onClicked: keysQrLoader.active = true
                        }

                        // Lazy-load. The dialog instantiates the QR image
                        // provider + a Window — we don't pay for either
                        // until the user explicitly opens it. show() runs
                        // from Component.onCompleted so the loaded item
                        // becomes visible on the same click.
                        Loader {
                            id: keysQrLoader
                            active: false
                            sourceComponent: Component {
                                KeysQRDialog {
                                    encKey: airPodsTrayApp.deviceInfo.magicAccEncKey
                                    irk: airPodsTrayApp.deviceInfo.magicAccIRK
                                    Component.onCompleted: show()
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
