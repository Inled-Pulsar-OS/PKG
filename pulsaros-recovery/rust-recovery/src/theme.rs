//! Estilos globales de la aplicación.
pub(crate) const APP_CSS: &str = r#"
/* Force macOS Dark Backdrop */
window, window.background, .background, .root-container {
    background-color: #1e1e20;
    color: #ffffff;
}
window, .root-container, * {
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
}
.apple-box {
    background-color: #323236;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 20px;
    padding: 24px 28px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7);
}
.welcome-title {
    font-size: 22px;
    font-weight: 700;
    color: #ffffff;
    margin-top: 4px;
    margin-bottom: 4px;
}
.welcome-subtitle {
    font-size: 13px;
    color: #c7c7cc;
    margin-bottom: 12px;
}
.info-card {
    background-color: transparent;
    border: none;
    padding: 0px;
    margin-top: 4px;
    margin-bottom: 8px;
}
.info-card-text {
    font-size: 13px;
    color: #e5e5ea;
    line-height: 1.5;
}
.setting-label {
    font-size: 12px;
    font-weight: 600;
    color: #a1a1a6;
    margin-bottom: 2px;
}
.badge-net-ok {
    color: #30d158;
    font-weight: 600;
    font-size: 12px;
}
.badge-net-warn {
    color: #ff9f0a;
    font-weight: 600;
    font-size: 12px;
}
.badge-net-err {
    color: #ff453a;
    font-weight: 600;
    font-size: 12px;
}
.wifi-badge-connected {
    background-color: rgba(48, 209, 88, 0.18);
    color: #30d158;
    border: 1px solid rgba(48, 209, 88, 0.4);
    border-radius: 6px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: bold;
}
.wifi-badge-security {
    background-color: rgba(255, 255, 255, 0.08);
    color: #c7c7cc;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 10px;
    font-weight: 500;
}
.wifi-card-row {
    background-color: transparent;
    border: none;
    border-radius: 8px;
    padding: 8px 12px;
    transition: background-color 0.15s ease;
}
listbox > row:hover .wifi-card-row,
listboxrow:hover .wifi-card-row {
    background-color: rgba(255, 255, 255, 0.08);
}
listbox > row:selected .wifi-card-row,
listboxrow:selected .wifi-card-row {
    background-color: #0071e3;
}
.wifi-ssid-text {
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
}
.wifi-signal-text {
    font-size: 12px;
    color: #a1a1a6;
}
.badge-demo {
    background-color: rgba(255, 159, 10, 0.18);
    color: #ff9f0a;
    border: 1px solid rgba(255, 159, 10, 0.4);
    border-radius: 6px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: bold;
}
/* Force completely transparent ListBox with clean seamless rows */
list, listview, listbox, .transparent-list, .content, .boxed-list {
    background-color: transparent;
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
}
listbox > row, listboxrow, row, .utility-item-row {
    background-color: transparent;
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
}
.utility-row-card {
    background-color: transparent;
    border: none;
    border-radius: 8px;
    padding: 8px 12px;
    transition: background-color 0.15s ease;
}
listbox > row:hover .utility-row-card,
listboxrow:hover .utility-row-card,
.utility-item-row:hover .utility-row-card {
    background-color: rgba(255, 255, 255, 0.08);
}
listbox > row:selected .utility-row-card,
listboxrow:selected .utility-row-card,
.utility-item-row:selected .utility-row-card {
    background-color: #0071e3;
}
.utility-title-lbl {
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
}
.utility-desc-lbl {
    font-size: 12px;
    color: #c7c7cc;
}
listbox > row:selected .utility-title-lbl,
listboxrow:selected .utility-title-lbl,
.utility-item-row:selected .utility-title-lbl {
    color: #ffffff;
}
listbox > row:selected .utility-desc-lbl,
listboxrow:selected .utility-desc-lbl,
.utility-item-row:selected .utility-desc-lbl {
    color: rgba(255, 255, 255, 0.92);
}
.suggested-action {
    background-color: #0071e3;
    color: #ffffff;
    border-radius: 10px;
    font-weight: 600;
    padding: 6px 24px;
    border: none;
    font-size: 13px;
}
.suggested-action:hover {
    background-color: #007bf5;
}
.suggested-action:disabled {
    background-color: #38383a;
    color: #636366;
}
.secondary-action {
    background-color: rgba(255, 255, 255, 0.08);
    color: #ffffff;
    border-radius: 10px;
    font-weight: 600;
    padding: 6px 20px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    font-size: 13px;
}
.secondary-action:hover {
    background-color: rgba(255, 255, 255, 0.14);
    border-color: rgba(255, 255, 255, 0.25);
}
.destructive-action {
    background-color: rgba(255, 69, 58, 0.15);
    color: #ff453a;
    border-radius: 10px;
    font-weight: 600;
    padding: 10px 20px;
    border: 1px solid rgba(255, 69, 58, 0.3);
    font-size: 13px;
}
.destructive-action:hover {
    background-color: rgba(255, 69, 58, 0.25);
}
.shortcut-btn {
    background-color: rgba(255, 255, 255, 0.08);
    color: #ffffff;
    border-radius: 6px;
    padding: 4px 10px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    font-size: 11px;
}
.shortcut-btn:hover {
    background-color: rgba(255, 255, 255, 0.15);
}
.progress-bar-thin {
    min-height: 8px;
    margin-top: 12px;
    margin-bottom: 12px;
}
.progress-bar-thin trough {
    min-height: 8px;
    border-radius: 9999px;
    background-color: #3a3a3c;
    border: none;
}
.progress-bar-thin progress {
    min-height: 8px;
    border-radius: 9999px;
    background-color: #0071e3;
    border: none;
}
.progress-text {
    font-size: 13px;
    color: #aeaeb2;
}
.live-log-view {
    background-color: #121212;
    border: 1px solid #333333;
    border-radius: 8px;
    padding: 6px;
}
.live-log-text text {
    background-color: #121212;
    color: #30d158;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
    font-size: 11px;
}
.err-log-text text {
    background-color: #121212;
    color: #ff453a;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
    font-size: 11px;
}
.disk-card {
    background-color: #2a2a2a;
    border: 1px solid #3c3c3c;
    border-radius: 12px;
    padding: 14px;
    min-width: 130px;
    margin: 6px;
    transition: all 0.15s ease;
}
.disk-card:hover {
    background-color: #323236;
}
.disk-card.selected {
    background-color: #323236;
    border: 2px solid #0071e3;
}
.bottom-power-btn {
    background-color: rgba(255, 255, 255, 0.08);
    color: #ffffff;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 20px;
    padding: 5px 24px;
    font-size: 13px;
    font-weight: 600;
    transition: all 0.15s ease;
}
.bottom-power-btn:hover {
    background-color: rgba(255, 255, 255, 0.16);
    border-color: rgba(255, 255, 255, 0.3);
}
"#;
