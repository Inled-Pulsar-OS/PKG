import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Qt5Compat.GraphicalEffects
import "components"

Rectangle {
    id: rootWindow
    width: parent ? parent.width : 1920
    height: parent ? parent.height : 1080
    color: "#18181b"
    LayoutMirroring.enabled: Qt.locale().textDirection === Qt.RightToLeft
    LayoutMirroring.childrenInherit: true
    property int sizeAvatar: 110
    property int longitudMasLarga: 0

    property int lastIndexUser: 0
    property string lastNameUser: users.lastNameUser
    property int implicitCustomWidth: 0
    property ListModel jUser: users.usersList
    property bool firtInteraction: true
    property bool startAnimationNames: false
    property bool isAuthenticating: false

    // Multi-monitor screen detection:
    // Determine if this view is the primary screen or a secondary screen
    readonly property bool isPrimaryScreen: {
        if (typeof screenModel === "undefined" || !screenModel || screenModel.count <= 1) {
            return true;
        }
        var pIdx = (screenModel.primary !== undefined && screenModel.primary >= 0) ? screenModel.primary : 0;
        var pGeom = screenModel.geometry(pIdx);
        if (typeof geometry !== "undefined" && geometry) {
            return (geometry.x === pGeom.x && geometry.y === pGeom.y);
        }
        return (rootWindow.x === pGeom.x);
    }

    TextConstants {
        id: textConstants
    }

    UserModel {
        id: users
    }

    function determinateNewIndex() {
        for (var j = 0; j < qmlUserModel.count; j++) {
            if (qmlUserModel.get(j).name === lastNameUser) {
                return j
            }
        }
        return 0
    }

    function submitLogin() {
        if (isAuthenticating) return
        isAuthenticating = true
        sddm.login(users.finalLoginUserName, password.text, session.currentIndex)
    }

    FontLoader {
        id: fontbold
        source: "fonts/SFUIText-Semibold.otf"
    }

    Loader {
        id: inputPanel
        property bool keyboardActive: false
        source: "components/VirtualKeyboard.qml"
    }

    Connections {
        target: sddm
        function onLoginSucceeded() {
            isAuthenticating = true
        }
        function onLoginFailed() {
            isAuthenticating = false
            password.placeholderText = textConstants.loginFailed
            password.placeholderTextColor = "#ff6b6b"
            password.text = ""
            password.focus = true
            shakeAnim.start()
        }
    }

    Item {
        id: wallpaperContainer
        anchors.fill: parent

        Image {
            id: wallpaper
            anchors.fill: parent
            fillMode: Image.PreserveAspectCrop
            visible: !animatedWallpaper.visible && !(videoLoader.item && videoLoader.item.hasVideo)
            cache: false
            source: (config.background !== undefined && !config.background.toString().endsWith(".mp4") && !config.background.toString().endsWith(".webm") && !config.background.toString().endsWith(".mkv") && !config.background.toString().endsWith(".mov"))
                    ? config.background
                    : (Qt.resolvedUrl("pulsar-os-tahoe.png"))
        }

        AnimatedImage {
            id: animatedWallpaper
            anchors.fill: parent
            fillMode: Image.PreserveAspectCrop
            visible: source.toString().endsWith(".gif") || source.toString().endsWith(".webp")
            cache: false
            playing: true
            source: config.background !== undefined ? config.background : ""
        }

        Loader {
            id: videoLoader
            anchors.fill: parent
            active: true
            source: "components/VideoWallpaper.qml"
        }
    }

    Image {
        id: staticBlurSource
        anchors.fill: parent
        fillMode: Image.PreserveAspectCrop
        source: (config.background !== undefined && !config.background.toString().endsWith(".mp4") && !config.background.toString().endsWith(".webm") && !config.background.toString().endsWith(".mkv") && !config.background.toString().endsWith(".mov"))
                ? config.background
                : "file:///var/lib/pulsar-sddm/pulsar-wallpaper.png"
        visible: false
        cache: false
    }

    // Secondary Screen Display: centered large liquid glass clock
    Item {
        id: secondaryScreenContainer
        anchors.fill: parent
        visible: !rootWindow.isPrimaryScreen

        Clock {
            anchors.centerIn: parent
            scale: 1.15
        }
    }

    // Primary Screen Container Area: Clock, User Login Card, Top Bar
    Item {
        id: primaryScreenArea
        anchors.fill: parent
        visible: rootWindow.isPrimaryScreen

        // Central Liquid Glass Clock
        Item {
            id: identclock
            anchors.top: parent.top
            anchors.topMargin: Math.max(40, parent.height * 0.12)
            anchors.horizontalCenter: parent.horizontalCenter
            width: clock.width
            height: clock.height

            Clock {
                id: clock
                anchors.centerIn: parent
            }
        }

        // User Login Card
        Rectangle {
            id: baseOfUserDialog
            width: listuser.visible ? (listuser.width > password.width ? listuser.width + (listuser.spacing * userModel.count) : password.width) : password.width
            height: listuser.visible ? (listuser.height + password.height + greetingLabel.height + password.height + 40) : (sizeAvatar + password.height + greetingLabel.height + password.height + 40)
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: Math.max(40, parent.height * 0.08)
            color: "transparent"

            SequentialAnimation {
                id: shakeAnim
                NumberAnimation { target: baseOfUserDialog; property: "anchors.horizontalCenterOffset"; from: 0; to: -14; duration: 40; easing.type: Easing.OutQuad }
                NumberAnimation { target: baseOfUserDialog; property: "anchors.horizontalCenterOffset"; from: -14; to: 14; duration: 60; easing.type: Easing.InOutQuad }
                NumberAnimation { target: baseOfUserDialog; property: "anchors.horizontalCenterOffset"; from: 14; to: -10; duration: 50; easing.type: Easing.InOutQuad }
                NumberAnimation { target: baseOfUserDialog; property: "anchors.horizontalCenterOffset"; from: -10; to: 10; duration: 50; easing.type: Easing.InOutQuad }
                NumberAnimation { target: baseOfUserDialog; property: "anchors.horizontalCenterOffset"; from: 10; to: 0; duration: 40; easing.type: Easing.OutQuad }
            }

            Item {
                id: sectionLogin
                height: parent.height
                width: parent.width

                ListView {
                    id: listuser
                    width: implicitCustomWidth + sizeAvatar * 0.9
                    height: ((sizeAvatar * 0.9) + 10) * userModel.count
                    model: jUser
                    verticalLayoutDirection: ListView.BottomToTop
                    anchors.left: parent.left
                    anchors.leftMargin: ((parent.width / 2) - (sizeAvatar * 0.9) / 2) + 5
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: usernametext.top
                    anchors.bottomMargin: -10
                    visible: false
                    currentIndex: userModel.lastIndex

                    delegate: Item {
                        height: sizeAvatar * 0.9
                        width: nameList.implicitWidth + height + contentFullUser.spacing

                        Rectangle {
                            id: hoverBg
                            anchors.fill: parent
                            anchors.margins: -4
                            color: "white"
                            opacity: mouseArea.containsMouse ? 0.15 : 0.0
                            radius: 8
                            Behavior on opacity {
                                NumberAnimation { duration: 150 }
                            }
                        }

                        Row {
                            id: contentFullUser
                            height: parent.height - 10
                            width: parent.width + spacing
                            spacing: 10
                            anchors.top: parent.top
                            Rectangle {
                                id: maskByList
                                width: sizeAvatar * 0.9
                                height: width
                                color: "black"
                                visible: false
                                radius: height / 2
                            }
                            Image {
                                id: avaList
                                source: (model.icon && model.icon.toString() !== "") ? model.icon : "images/.face.icon"
                                height: parent.height
                                width: height
                                fillMode: Image.PreserveAspectFit
                                layer.enabled: true
                                layer.effect: OpacityMask {
                                    maskSource: maskByList
                                }
                            }

                            Text {
                                id: nameList
                                text: model.name
                                color: "white"
                                font.bold: true
                                visible: !startAnimationNames
                                font.pixelSize: 14
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }

                        Rectangle {
                            id: resalt
                            color: "#ff991c"
                            width: parent.width / 3.5
                            height: width
                            radius: width / 2
                            border.color: "white"
                            border.width: width / 14
                            visible: model.name === userModel.currentText
                            anchors.bottom: parent.bottom
                            anchors.right: parent.right
                            Image {
                                id: palomita
                                anchors.horizontalCenter: parent.horizontalCenter
                                width: parent.width * 0.6
                                height: width
                                source: "images/palomita.svg"
                                sourceSize: Qt.size(width, width)
                                fillMode: Image.PreserveAspectFit
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }

                        MouseArea {
                            id: mouseArea
                            anchors.fill: contentFullUser
                            hoverEnabled: true
                            onClicked: {
                                listuser.visible = !listuser.visible
                                ava.visible = !ava.visible
                                users.lastNameUser = nameList.text
                            }
                        }
                        Component.onCompleted: {
                            implicitCustomWidth = nameList.implicitWidth > implicitCustomWidth ? nameList.implicitWidth : implicitCustomWidth
                        }
                    }

                    Behavior on opacity {
                        NumberAnimation {
                            duration: 300
                            easing.type: Easing.InOutQuad
                        }
                    }

                    states: [
                        State {
                            name: "visible"
                            when: listuser.visible
                            PropertyChanges {
                                target: listuser
                                opacity: 1
                            }
                        },
                        State {
                            name: "hidden"
                            when: !listuser.visible
                            PropertyChanges {
                                target: listuser
                                opacity: 0
                            }
                        }
                    ]

                    transitions: [
                        Transition {
                            from: "hidden"
                            to: "visible"
                            NumberAnimation {
                                target: listuser
                                property: "y"
                                duration: 300
                                easing.type: Easing.OutBounce
                                from: listuser.y + 20
                                to: listuser.y
                            }
                        },
                        Transition {
                            from: "visible"
                            to: "hidden"
                            NumberAnimation {
                                target: listuser
                                property: "y"
                                duration: 300
                                easing.type: Easing.InQuad
                                from: listuser.y
                                to: listuser.y + 20
                            }
                        }
                    ]
                }

                Rectangle {
                    id: mask
                    width: sizeAvatar
                    height: sizeAvatar
                    radius: sizeAvatar / 2
                    visible: false
                }

                DropShadow {
                    anchors.fill: ava
                    width: mask.width - 4
                    height: mask.height - 4
                    anchors.horizontalCenter: parent.horizontalCenter
                    horizontalOffset: 0
                    verticalOffset: 2
                    radius: 15.0
                    samples: 15
                    color: "#50000000"
                    source: mask
                    visible: listuser.visible ? false : true
                }

                Image {
                    id: ava
                    width: sizeAvatar
                    height: sizeAvatar
                    visible: true
                    fillMode: Image.PreserveAspectCrop
                    anchors.horizontalCenter: parent.horizontalCenter

                    layer.enabled: true
                    layer.effect: OpacityMask {
                        maskSource: mask
                    }
                    source: (users.lastUrlAvatar && users.lastUrlAvatar.toString() !== "") ? users.lastUrlAvatar : "images/.face.icon"
                    MouseArea {
                        anchors.fill: ava
                        onClicked: {
                            listuser.visible = true
                            ava.visible = false
                        }
                    }
                }

                Text {
                    id: usernametext
                    text: users.finalLoginUserName
                    anchors.top: parent.top
                    anchors.topMargin: sizeAvatar + 16
                    anchors.horizontalCenter: parent.horizontalCenter
                    font.pixelSize: 20
                    font.family: fontbold.name
                    font.capitalization: Font.Capitalize
                    font.weight: Font.DemiBold
                    visible: listuser.visible ? false : true
                    color: "white"
                    layer.enabled: true
                    layer.effect: DropShadow {
                        horizontalOffset: 1
                        verticalOffset: 2
                        radius: 10
                        samples: 25
                        color: "#33000000"
                    }
                }

                Text {
                    id: demo
                    text: textConstants.password
                    font.weight: Font.DemiBold
                    visible: false
                }

                // Apple Activity Spinner shown during unlock / login authentication
                AppleSpinner {
                    id: authSpinner
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: greetingLabel.height * 2 + (password.height - height) / 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    implicitWidth: 32
                    implicitHeight: 32
                    visible: isAuthenticating && !listuser.visible
                    running: visible
                }

                TextField {
                    id: password
                    property var vtext: TextInput.Password

                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: greetingLabel.height * 2
                    height: 32
                    width: 250
                    color: "#fff"
                    placeholderTextColor: "#66FFFFFF"
                    echoMode: TextInput.Password
                    focus: true
                    font.weight: Font.DemiBold
                    placeholderText: textConstants.password
                    leftPadding: (width - demo.implicitWidth) / 2
                    visible: (!listuser.visible) && (!isAuthenticating)

                    onAccepted: submitLogin()

                    background: Rectangle {
                        implicitWidth: parent.width
                        implicitHeight: parent.height
                        color: "#fff"
                        opacity: 0.2
                        radius: 16
                        border.color: Qt.rgba(1, 1, 1, 0.25)
                        border.width: 1
                    }

                    Image {
                        id: caps
                        width: 24
                        height: 24
                        opacity: 0
                        state: keyboard.capsLock ? "activated" : ""
                        anchors.right: password.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.rightMargin: 10
                        fillMode: Image.PreserveAspectFit
                        source: "images/capslock.svg"
                        sourceSize.width: 24
                        sourceSize.height: 24

                        states: [
                            State {
                                name: "activated"
                                PropertyChanges {
                                    target: caps
                                    opacity: 1
                                }
                            },
                            State {
                                name: ""
                                PropertyChanges {
                                    target: caps
                                    opacity: 0
                                }
                            }
                        ]

                        transitions: [
                            Transition {
                                to: "activated"
                                NumberAnimation {
                                    target: caps
                                    property: "opacity"
                                    from: 0
                                    to: 1
                                    duration: 200
                                }
                            },
                            Transition {
                                to: ""
                                NumberAnimation {
                                    target: caps
                                    property: "opacity"
                                    from: 1
                                    to: 0
                                    duration: 200
                                }
                            }
                        ]
                    }
                }

                Label {
                    id: greetingLabel
                    text: isAuthenticating ? "" : textConstants.promptPassword
                    color: "#fff"
                    style: Text.Normal
                    visible: (!listuser.visible) && (!isAuthenticating)
                    styleColor: "transparent"
                    font.pointSize: 8
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    font.bold: true
                }
            }

            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                    submitLogin()
                    event.accepted = true
                }
            }
        }

        // Top Bar (Session & Power Actions)
        Row {
            id: topBar
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.rightMargin: 40
            anchors.topMargin: 15
            spacing: 15
            z: 1000

            ComboBox {
                id: session
                height: 22
                width: 150
                model: sessionModel
                textRole: "name"
                displayText: ""
                currentIndex: sessionModel.lastIndex
                background: Rectangle {
                    implicitWidth: parent.width
                    implicitHeight: parent.height
                    color: "transparent"
                }

                delegate: MenuItem {
                    id: menuitems
                    width: slistview.width * 4
                    text: session.textRole ? (Array.isArray(session.model) ? modelData[session.textRole] : model[session.textRole]) : modelData
                    highlighted: session.highlightedIndex === index
                    hoverEnabled: session.hoverEnabled
                    onClicked: {
                        ava.source = "/var/lib/AccountsService/icons/" + user.currentText
                        session.currentIndex = index
                        slistview.currentIndex = index
                        session.popup.close()
                    }
                }
                indicator: Rectangle {
                    anchors.right: parent.right
                    anchors.rightMargin: 9
                    height: parent.height
                    width: 22
                    color: "transparent"
                    Image {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width
                        height: width
                        fillMode: Image.PreserveAspectFit
                        source: "images/conf.svg"
                    }
                }
                popup: Popup {
                    width: parent.width
                    height: Math.min(slistview.contentHeight, 200)
                    implicitHeight: slistview.contentHeight
                    margins: 0
                    contentItem: ListView {
                        id: slistview
                        clip: true
                        anchors.fill: parent
                        model: session.model
                        spacing: 0
                        highlightFollowsCurrentItem: true
                        currentIndex: session.highlightedIndex
                        delegate: session.delegate
                    }
                }
            }

            Image {
                id: reboot
                height: 22
                width: 22
                source: "images/system-reboot.svg"
                fillMode: Image.PreserveAspectFit

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    onEntered: {
                        reboot.source = "images/system-reboot-hover.svg"
                        var component = Qt.createComponent("components/RebootToolTip.qml")
                        if (component.status === Component.Ready) {
                            var tooltip = component.createObject(reboot)
                            tooltip.x = -100
                            tooltip.y = 40
                            tooltip.destroy(600)
                        }
                    }
                    onExited: {
                        reboot.source = "images/system-reboot.svg"
                    }
                    onClicked: {
                        reboot.source = "images/system-reboot-pressed.svg"
                        sddm.reboot()
                    }
                }
            }

            Image {
                id: shutdown
                height: 22
                width: 22
                source: "images/system-shutdown.svg"
                fillMode: Image.PreserveAspectFit

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    onEntered: {
                        shutdown.source = "images/system-shutdown-hover.svg"
                        var component = Qt.createComponent("components/ShutdownToolTip.qml")
                        if (component.status === Component.Ready) {
                            var tooltip = component.createObject(shutdown)
                            tooltip.x = -100
                            tooltip.y = 40
                            tooltip.destroy(600)
                        }
                    }
                    onExited: {
                        shutdown.source = "images/system-shutdown.svg"
                    }
                    onClicked: {
                        shutdown.source = "images/system-shutdown-pressed.svg"
                        sddm.powerOff()
                    }
                }
            }
        }
    }
}
