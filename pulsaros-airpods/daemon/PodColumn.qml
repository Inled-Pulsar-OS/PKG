import QtQuick 2.15
import linux 1.0

Column {
    id: root

    property bool inEar: true
    property string iconSource
    property int batteryLevel: 0
    property bool isCharging: false
    property string indicator: ""
    // 2D glyph path (Nerd Font mdi-headphones family) gated behind
    // `use2D`. Default OFF: the generic mdi-headphones glyph is the
    // same for every AirPods model, which user-tested as worse than
    // the per-model product photos because it loses model identity.
    // Path is kept for a future tick that ships proper per-model SVG
    // outlines (one per AirPods, AirPods 2, AirPods 3, AirPods 4,
    // AirPods Pro, AirPods Pro 2, AirPods Pro 3, AirPods Max).
    property bool use2D: false

    spacing: 5
    // Direct binding + Behavior replaces the previous Timer-driven
    // opacityTimer.restart() pattern, which deferred the change 50ms for
    // no obvious reason. 180ms OutCubic matches Quickshell AudioControls.
    opacity: inEar ? 1 : 0.5

    // Pick the glyph that best matches the rendered pod/case. Material
    // Design Icons ship through JetBrainsMono Nerd Font (already
    // Theme.iconFont). Case uses headphones-box; max uses headphones-
    // bluetooth; everything else uses headphones (matches the AirPods
    // bud silhouette closer than the in-ear-monitor variant).
    readonly property string glyph: {
        // Codepoints from Material Design Icons mapped into the Nerd
        // Font PUA range (U+F0000+). MDI 6+ codepoints; lookups via
        // https://www.nerdfonts.com/cheat-sheet (filter "md-headphones").
        if (indicator === "Case")
            return "\u{f02d2}";          // mdi-headphones-box
        if (iconSource.indexOf("podmax") !== -1)
            return "\u{f0f26}"; // mdi-headphones-bluetooth
        return "\u{f02cb}";                                    // mdi-headphones
    }

    // 2D glyph variant: single Nerd-Font character at large size. Same
    // visual footprint as the photo for layout stability; no new asset
    // shipped because the font is system-installed.
    Text {
        visible: root.use2D
        text: root.glyph
        font.family: Theme.iconFont
        font.pixelSize: root.indicator === "" ? 80 : 64
        color: Theme.colFg
        width: root.indicator === "" ? 92 : 72
        height: 72
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        anchors.horizontalCenter: parent.horizontalCenter
        renderType: Text.NativeRendering
    }

    // Legacy photo asset path. Kept for the fallback when use2D is
    // false — lets a future settings toggle flip back to the
    // skeuomorphic look without re-introducing the assets via QRC.
    Image {
        visible: !root.use2D
        source: root.iconSource
        width: root.indicator === "" ? 92 : 72
        height: 72
        fillMode: Image.PreserveAspectFit
        mipmap: true
        mirror: root.indicator === "R"
        anchors.horizontalCenter: parent.horizontalCenter
    }

    BatteryIndicator {
        batteryLevel: root.batteryLevel
        isCharging: root.isCharging
        indicator: root.indicator
    }

    Behavior on opacity {
        NumberAnimation {
            duration: 180
            easing.type: Easing.OutCubic
        }
    }
}
