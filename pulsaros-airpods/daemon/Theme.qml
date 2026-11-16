pragma Singleton
import QtQuick 2.15

QtObject {
    readonly property color colBg: "#0d0d0d"
    readonly property color colFg: "#ffffff"
    readonly property color colDim: "#8d8d8d"
    readonly property color colAccent: "#b6b6b6"
    readonly property color colBorder: "#2a2a2a"
    readonly property color colToggleOn: "#ffffff"
    readonly property color colToggleOff: "#1a1a1a"
    readonly property color colUrgent: "#a55555"
    readonly property color colOnline: "#6fdc6f"
    readonly property color colWarning: "#cecece"
    // Semantic aliases over canonical palette so OpenPods tracks Quickshell
    // Theme.qml as single source of truth. Card surface = toggle-off tier;
    // battery-low = urgent, mid = warning, charging/high = online. No
    // duplicate hex values.
    readonly property color colSurface: colToggleOff
    readonly property color colCharging: colOnline
    readonly property color colBatLow: colUrgent
    readonly property color colBatMid: colWarning
    readonly property color colBatHigh: colOnline
    readonly property string fontFamily: "SF Pro Text"
    // Mirror Quickshell's iconFont so chrome glyphs (settings, back, etc.)
    // render in the same Nerd Font the bar uses, not the bundled SF Symbols
    // PUA glyphs which look out of place next to Quickshell widgets.
    readonly property string iconFont: "JetBrainsMono Nerd Font"
    readonly property int fontSize: 14
    readonly property int radius: 12
    readonly property int tileRadius: 8
    readonly property int borderWidth: 1
}
