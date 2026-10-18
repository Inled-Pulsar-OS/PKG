import QtQuick
import QtQuick.Controls

Item {
    id: bioRoot
    width: 320
    height: 76
    visible: hasFace || hasFingerprint

    property string targetUser: ""
    property bool hasFace: false
    property bool hasFingerprint: false
    property string faceState: "paused"     // "idle", "paused", "scanning", "matched", "failed"
    property string fpState: "idle"         // "idle", "waiting", "matched", "failed"
    property string statusText: ""
    property bool authTriggered: false

    signal biometricsSuccess()

    property int faceFailCount: 0

    function getEffectiveUser() {
        if (targetUser && targetUser !== "") return targetUser;
        if (typeof users !== "undefined" && users && users.finalLoginUserName && users.finalLoginUserName !== "") return users.finalLoginUserName;
        if (typeof userModel !== "undefined" && userModel && userModel.lastUser && userModel.lastUser !== "") return userModel.lastUser;
        return "";
    }

    // ── HTTP Communication with pulsaros-biometrics-daemon ──────────
    function fetchStatus() {
        var user = getEffectiveUser();
        var xhr = new XMLHttpRequest();
        xhr.open("GET", "http://127.0.0.1:39281/status" + (user !== "" ? ("?user=" + encodeURIComponent(user)) : ""));
        xhr.onreadystatechange = function() {
            if (xhr.readyState === XMLHttpRequest.DONE && xhr.status === 200) {
                try {
                    var data = JSON.parse(xhr.responseText);
                    if (data.user && data.user !== "") {
                        if (!bioRoot.targetUser || bioRoot.targetUser === "") {
                            bioRoot.targetUser = data.user;
                        }
                        if (typeof users !== "undefined" && users && (!users.finalLoginUserName || users.finalLoginUserName === "")) {
                            users.finalLoginUserName = data.user;
                        }
                    }
                    bioRoot.hasFace = (data.face_enrolled === true);
                    bioRoot.hasFingerprint = (data.fingerprint_enrolled === true) || (data.has_fingerprint === true);

                    if (data.face_state !== bioRoot.faceState) {
                        bioRoot.faceState = data.face_state;
                        if (data.face_state === "failed") {
                            bioRoot.faceFailCount += 1;
                            if (bioRoot.faceFailCount >= 2) {
                                var effUser = bioRoot.getEffectiveUser();
                                var cXhr = new XMLHttpRequest();
                                cXhr.open("GET", "http://127.0.0.1:39281/show_camera" + (effUser !== "" ? ("?user=" + encodeURIComponent(effUser)) : ""));
                                cXhr.send();
                            }
                        } else if (data.face_state === "matched") {
                            bioRoot.faceFailCount = 0;
                        }
                    }
                    if (data.fingerprint_state !== bioRoot.fpState) {
                        bioRoot.fpState = data.fingerprint_state;
                    }
                    if (data.status_message && data.status_message !== "") {
                        bioRoot.statusText = data.status_message;
                    }

                    // Check for match success
                    if ((data.face_state === "matched" || data.fingerprint_state === "matched") && !bioRoot.authTriggered) {
                        bioRoot.authTriggered = true;
                        successTimer.start();
                    }
                } catch (e) {}
            }
        };
        xhr.send();
    }

    function startVerification(autoFace) {
        var user = getEffectiveUser();
        bioRoot.authTriggered = false;
        var faceParam = (autoFace === true) ? "1" : "0";
        var xhr = new XMLHttpRequest();
        xhr.open("POST", "http://127.0.0.1:39281/verify/start?source=sddm&face=" + faceParam + (user !== "" ? ("&user=" + encodeURIComponent(user)) : ""));
        xhr.send();
    }

    function startFaceVerification() {
        var user = getEffectiveUser();
        bioRoot.faceState = "scanning";
        var isEs = Qt.locale().name.indexOf("es") === 0;
        bioRoot.statusText = isEs ? "Buscando rostro..." : "Looking for face...";
        var xhr = new XMLHttpRequest();
        xhr.open("POST", "http://127.0.0.1:39281/verify/activate_face" + (user !== "" ? ("?user=" + encodeURIComponent(user)) : ""));
        xhr.send();
    }

    Timer {
        id: pollTimer
        interval: 200
        running: true
        repeat: true
        onTriggered: bioRoot.fetchStatus()
    }

    Timer {
        id: successTimer
        interval: 420
        repeat: false
        onTriggered: {
            bioRoot.biometricsSuccess();
        }
    }

    onTargetUserChanged: {
        bioRoot.authTriggered = false;
        startVerification(false);
        fetchStatus();
    }

    Component.onCompleted: {
        // Auto-activate Face ID on greeter start (cold boot AND after logout):
        // the daemon scans immediately instead of waiting for a click.
        startVerification(true);
        fetchStatus();
    }

    // ── UI Components Layout ─────────────────────────────────────────
    Column {
        anchors.centerIn: parent
        spacing: 6

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 20

            // ── 1. WINDOWS HELLO FACE ANIMATION ──────────────────────
            Item {
                id: helloFaceContainer
                width: 44
                height: 44
                visible: bioRoot.hasFace

                property real scanAngle: 0.0
                property real eyeXOffset: 0.0
                property real eyeYOffset: 0.0
                property real winkScale: 1.0

                // Scan spinning arc animation
                NumberAnimation {
                    id: scanSpinAnim
                    target: helloFaceContainer
                    property: "scanAngle"
                    from: 0
                    to: 360
                    duration: 1100
                    loops: Animation.Infinite
                    running: bioRoot.faceState === "scanning"
                }

                // Eyes glancing around animation
                SequentialAnimation {
                    id: eyesLookAnim
                    loops: Animation.Infinite
                    running: bioRoot.faceState === "scanning"

                    NumberAnimation { target: helloFaceContainer; property: "eyeXOffset"; to: -2.8; duration: 500; easing.type: Easing.InOutQuad }
                    PauseAnimation { duration: 180 }
                    NumberAnimation { target: helloFaceContainer; property: "eyeXOffset"; to: 2.8; duration: 600; easing.type: Easing.InOutQuad }
                    PauseAnimation { duration: 180 }
                    NumberAnimation { target: helloFaceContainer; property: "eyeXOffset"; to: 0.0; duration: 350; easing.type: Easing.InOutQuad }
                    NumberAnimation { target: helloFaceContainer; property: "eyeYOffset"; to: -1.5; duration: 400; easing.type: Easing.InOutQuad }
                    PauseAnimation { duration: 250 }
                    NumberAnimation { target: helloFaceContainer; property: "eyeYOffset"; to: 0.0; duration: 350; easing.type: Easing.InOutQuad }
                    PauseAnimation { duration: 600 }
                }

                // Idle / Paused subtle breathing animation
                SequentialAnimation {
                    id: pausedBreatheAnim
                    running: bioRoot.faceState === "paused"
                    loops: Animation.Infinite
                    NumberAnimation { target: helloFaceContainer; property: "opacity"; from: 0.70; to: 1.0; duration: 1100; easing.type: Easing.InOutQuad }
                    NumberAnimation { target: helloFaceContainer; property: "opacity"; from: 1.0; to: 0.70; duration: 1100; easing.type: Easing.InOutQuad }
                }

                // Click to activate Face ID
                MouseArea {
                    anchors.fill: parent
                    cursorShape: (bioRoot.faceState === "paused" || bioRoot.faceState === "idle") ? Qt.PointingHandCursor : Qt.ArrowCursor
                    onClicked: {
                        if (bioRoot.faceState === "paused" || bioRoot.faceState === "idle") {
                            bioRoot.startFaceVerification();
                        }
                    }
                }

                // Success Wink Animation
                SequentialAnimation {
                    id: winkAnim
                    running: bioRoot.faceState === "matched"
                    NumberAnimation { target: helloFaceContainer; property: "winkScale"; to: 0.15; duration: 120; easing.type: Easing.InOutQuad }
                    NumberAnimation { target: helloFaceContainer; property: "winkScale"; to: 1.0; duration: 140; easing.type: Easing.InOutQuad }
                }

                // Denial Shake Animation
                SequentialAnimation {
                    id: faceShakeAnim
                    running: bioRoot.faceState === "failed"
                    loops: 3
                    NumberAnimation { target: helloFaceContainer; property: "x"; from: 0; to: -6; duration: 40 }
                    NumberAnimation { target: helloFaceContainer; property: "x"; to: 6; duration: 40 }
                    NumberAnimation { target: helloFaceContainer; property: "x"; to: 0; duration: 40 }
                }

                Canvas {
                    id: helloFaceCanvas
                    anchors.fill: parent

                    Connections {
                        target: helloFaceContainer
                        function onScanAngleChanged() { helloFaceCanvas.requestPaint(); }
                        function onEyeXOffsetChanged() { helloFaceCanvas.requestPaint(); }
                        function onEyeYOffsetChanged() { helloFaceCanvas.requestPaint(); }
                        function onWinkScaleChanged() { helloFaceCanvas.requestPaint(); }
                    }

                    Connections {
                        target: bioRoot
                        function onFaceStateChanged() { helloFaceCanvas.requestPaint(); }
                    }

                    onPaint: {
                        var ctx = getContext("2d");
                        ctx.clearRect(0, 0, width, height);

                        var cx = width / 2;
                        var cy = height / 2;
                        var r = 16.5;

                        var mainColor = (bioRoot.faceState === "matched") ? "#30d158" :
                                        (bioRoot.faceState === "failed") ? "#ff453a" : "#ffffff";

                        ctx.strokeStyle = mainColor;
                        ctx.fillStyle = mainColor;
                        ctx.lineWidth = 3.0;
                        ctx.lineCap = "round";

                        if (bioRoot.faceState === "matched") {
                            // Warm Smile arc
                            ctx.beginPath();
                            ctx.arc(cx, cy - 1, 14.0, Math.PI * 0.22, Math.PI * 0.78, false);
                            ctx.stroke();

                            // Left eye: open dot
                            ctx.beginPath();
                            ctx.arc(cx - 7.0, cy - 4.0, 2.5, 0, Math.PI * 2, false);
                            ctx.fill();

                            // Right eye: wink!
                            if (helloFaceContainer.winkScale < 0.4) {
                                ctx.beginPath();
                                ctx.moveTo(cx + 4.0, cy - 4.0);
                                ctx.lineTo(cx + 10.0, cy - 4.0);
                                ctx.stroke();
                            } else {
                                ctx.beginPath();
                                ctx.arc(cx + 7.0, cy - 4.0, 2.5 * helloFaceContainer.winkScale, 0, Math.PI * 2, false);
                                ctx.fill();
                            }

                        } else if (bioRoot.faceState === "failed") {
                            // Frown arc cleanly separated below eyes
                            ctx.beginPath();
                            ctx.arc(cx, cy + 10, 8.5, Math.PI * 1.18, Math.PI * 1.82, false);
                            ctx.stroke();

                            // Downcast / sad eyes separated from frown
                            ctx.beginPath();
                            ctx.arc(cx - 7.0, cy - 4.0, 2.5, 0, Math.PI * 2, false);
                            ctx.arc(cx + 7.0, cy - 4.0, 2.5, 0, Math.PI * 2, false);
                            ctx.fill();

                        } else if (bioRoot.faceState === "paused") {
                            // Resting Windows Hello face: steady ring and calm centered eyes + mouth
                            ctx.strokeStyle = Qt.rgba(1, 1, 1, 0.78);
                            ctx.beginPath();
                            ctx.arc(cx, cy, r, 0, Math.PI * 2, false);
                            ctx.stroke();

                            ctx.fillStyle = Qt.rgba(1, 1, 1, 0.88);
                            ctx.beginPath();
                            ctx.arc(cx - 7.0, cy - 4.0, 2.5, 0, Math.PI * 2, false);
                            ctx.arc(cx + 7.0, cy - 4.0, 2.5, 0, Math.PI * 2, false);
                            ctx.fill();

                            ctx.beginPath();
                            ctx.arc(cx, cy + 2, 8.0, Math.PI * 0.22, Math.PI * 0.78, false);
                            ctx.stroke();

                        } else {
                            // Scanning: Rotating Windows Hello arc
                            var rad = (helloFaceContainer.scanAngle * Math.PI) / 180.0;
                            var arcSpan = Math.PI * 0.78; // ~140 degrees arc

                            ctx.beginPath();
                            ctx.arc(cx, cy, r, rad, rad + arcSpan, false);
                            ctx.stroke();

                            // Eyes looking around
                            var ex = helloFaceContainer.eyeXOffset;
                            var ey = helloFaceContainer.eyeYOffset;

                            ctx.beginPath();
                            ctx.arc(cx - 7.0 + ex, cy - 4.0 + ey, 2.5, 0, Math.PI * 2, false);
                            ctx.arc(cx + 7.0 + ex, cy - 4.0 + ey, 2.5, 0, Math.PI * 2, false);
                            ctx.fill();
                        }
                    }
                }
            }

            // ── 2. TOUCH ID FINGERPRINT ANIMATION ────────────────────
            Item {
                id: fpContainer
                width: 44
                height: 44
                visible: bioRoot.hasFingerprint

                // Pulsing breathing animation while waiting
                SequentialAnimation {
                    id: fpPulseAnim
                    running: bioRoot.fpState === "waiting" || bioRoot.fpState === "idle"
                    loops: Animation.Infinite
                    NumberAnimation { target: fpContainer; property: "opacity"; from: 0.45; to: 1.0; duration: 900; easing.type: Easing.InOutQuad }
                    NumberAnimation { target: fpContainer; property: "opacity"; from: 1.0; to: 0.45; duration: 900; easing.type: Easing.InOutQuad }
                }

                // Denial Shake
                SequentialAnimation {
                    id: fpShakeAnim
                    running: bioRoot.fpState === "failed"
                    loops: 3
                    NumberAnimation { target: fpContainer; property: "x"; from: 0; to: -6; duration: 40 }
                    NumberAnimation { target: fpContainer; property: "x"; to: 6; duration: 40 }
                    NumberAnimation { target: fpContainer; property: "x"; to: 0; duration: 40 }
                }

                // GNOME fingerprint icon (same as the lock screen's fingerprint-symbolic)
                Image {
                    id: fpIconImg
                    anchors.centerIn: parent
                    width: 40
                    height: 40
                    source: "../images/fingerprint-symbolic.svg"
                    sourceSize.width: 40
                    sourceSize.height: 40
                    visible: bioRoot.fpState !== "matched"
                }

                Canvas {
                    id: fpCanvas
                    anchors.fill: parent
                    visible: bioRoot.fpState === "matched"

                    Connections {
                        target: bioRoot
                        function onFpStateChanged() { fpCanvas.requestPaint(); }
                    }

                    onPaint: {
                        var ctx = getContext("2d");
                        ctx.clearRect(0, 0, width, height);

                        var cx = width / 2;
                        var cy = height / 2;

                        // Green success circle with checkmark
                        ctx.fillStyle = "#30d158";
                        ctx.beginPath();
                        ctx.arc(cx, cy, 16, 0, Math.PI * 2, false);
                        ctx.fill();

                        ctx.strokeStyle = "#ffffff";
                        ctx.lineWidth = 3.0;
                        ctx.lineCap = "round";
                        ctx.lineJoin = "round";
                        ctx.beginPath();
                        ctx.moveTo(cx - 6, cy);
                        ctx.lineTo(cx - 2, cy + 4.5);
                        ctx.lineTo(cx + 6, cy - 4.5);
                        ctx.stroke();
                    }
                }
            }
        }

        // ── 3. STATUS BADGE ──────────────────────────────────────────
        Rectangle {
            id: statusBadge
            anchors.horizontalCenter: parent.horizontalCenter
            height: 20
            width: statusLabel.implicitWidth + 18
            radius: 10
            color: (bioRoot.faceState === "matched" || bioRoot.fpState === "matched") ? Qt.rgba(48/255, 209/255, 88/255, 0.35) :
                   (bioRoot.faceState === "failed" || bioRoot.fpState === "failed") ? Qt.rgba(255/255, 69/255, 58/255, 0.35) :
                   Qt.rgba(0, 0, 0, 0.38)
            border.color: (bioRoot.faceState === "matched" || bioRoot.fpState === "matched") ? "#30d158" :
                          (bioRoot.faceState === "failed" || bioRoot.fpState === "failed") ? "#ff453a" :
                          Qt.rgba(1, 1, 1, 0.22)
            border.width: 1

            Behavior on color {
                ColorAnimation { duration: 200 }
            }

            property bool isSpanish: Qt.locale().name.indexOf("es") === 0

            MouseArea {
                anchors.fill: parent
                cursorShape: (bioRoot.faceState === "paused" || bioRoot.faceState === "idle") ? Qt.PointingHandCursor : Qt.ArrowCursor
                onClicked: {
                    if (bioRoot.faceState === "paused" || bioRoot.faceState === "idle") {
                        bioRoot.startFaceVerification();
                    }
                }
            }

            Text {
                id: statusLabel
                anchors.centerIn: parent
                text: {
                    if (bioRoot.faceState === "matched" || bioRoot.fpState === "matched") {
                        return isSpanish ? "¡Identidad verificada!" : "Identity verified!";
                    }
                    if (bioRoot.faceState === "failed" && bioRoot.fpState === "failed") {
                        return isSpanish ? "No reconocido. Reintenta." : "Not recognized. Try again.";
                    }
                    if (bioRoot.faceState === "paused" || bioRoot.faceState === "idle") {
                        if (bioRoot.hasFingerprint) {
                            return isSpanish ? "Pulsa para activar Face ID o usa la huella" : "Click to activate Face ID or use fingerprint";
                        } else {
                            return isSpanish ? "Pulsa para activar Face ID" : "Click to activate Face ID";
                        }
                    }
                    if (bioRoot.statusText !== "") return bioRoot.statusText;
                    if (bioRoot.hasFace && bioRoot.hasFingerprint) {
                        return isSpanish ? "Esperando rostro o huella..." : "Waiting for face or fingerprint...";
                    }
                    if (bioRoot.hasFace) {
                        return isSpanish ? "Buscando rostro..." : "Looking for face...";
                    }
                    if (bioRoot.hasFingerprint) {
                        return isSpanish ? "Esperando huella..." : "Waiting for fingerprint...";
                    }
                    return "";
                }
                font.pixelSize: 11
                font.bold: true
                color: (bioRoot.faceState === "matched" || bioRoot.fpState === "matched") ? "#4cd964" :
                       (bioRoot.faceState === "failed" || bioRoot.fpState === "failed") ? "#ff6b6b" :
                       Qt.rgba(1, 1, 1, 0.9)
            }
        }
    }
}
