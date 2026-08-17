/**
 * Pulsar OS - Global Menu Extension
 * Extension to display macOS-like application menus in GNOME top bar.
 * 
 * Extensión de Pulsar OS - Menú Global
 * Extensión para mostrar menús de aplicación al estilo macOS en la barra superior de GNOME.
 * 
 * Compatible with GNOME 45-50 & Wayland.
 * Compatible con GNOME 45-50 y Wayland.
 */

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as ModalDialog from 'resource:///org/gnome/shell/ui/modalDialog.js';
import { getPointerWatcher } from 'resource:///org/gnome/shell/ui/pointerWatcher.js';
import * as SystemActions from 'resource:///org/gnome/shell/misc/systemActions.js';

import St from 'gi://St';
import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Shell from 'gi://Shell';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';
import Gst from 'gi://Gst';
import GstApp from 'gi://GstApp';

// Custom GObject class to construct the menu buttons in the panel
// Clase GObject personalizada para construir los botones de menú en el panel
const GlobalMenuButton = GObject.registerClass({
    GTypeName: 'PulsarosGlobalMenuButton'
}, class GlobalMenuButton extends PanelMenu.Button {
    _init(title, isAppMenu = false) {
        super._init(0.0, title, false);
        
        // St.Label widget to show the menu text on the top bar
        // Widget St.Label para mostrar el texto del menú en la barra superior
        this.label = new St.Label({
            text: title,
            y_align: Clutter.ActorAlign.CENTER,
            style_class: isAppMenu ? 'global-menu-app-label' : 'global-menu-label'
        });
        
        this.add_child(this.label);
    }
    
    // Updates the label text dynamically (e.g., when active window changes)
    // Actualiza el texto de la etiqueta dinámicamente (p. ej., al cambiar la ventana activa)
    setText(text) {
        this.label.set_text(text);
    }
});

const AboutDialog = GObject.registerClass({
    GTypeName: 'PulsarosAboutDialog'
}, class AboutDialog extends ModalDialog.ModalDialog {
    _init(osName, osVersion, hostName, cpuModel, memTotal, gpuModel, diskInfo) {
        super._init({ styleClass: 'pulsaros-about-dialog' });

        let mainBox = new St.BoxLayout({
            vertical: true,
            style_class: 'pulsaros-about-mainbox'
        });
        this.contentLayout.add_child(mainBox);

        // Add a nice Logo at the top
        let logoTexture = new St.Icon({
            icon_name: 'pulsar-logo',
            icon_size: 96,
            style_class: 'pulsaros-about-logo',
            x_align: Clutter.ActorAlign.CENTER
        });
        
        let logoBox = new St.BoxLayout({
            style_class: 'pulsaros-about-logobox',
            x_align: Clutter.ActorAlign.CENTER
        });
        logoBox.add_child(logoTexture);
        mainBox.add_child(logoBox);

        // Title
        let titleLabel = new St.Label({
            text: osName || "Pulsar OS",
            style_class: 'pulsaros-about-title',
            x_align: Clutter.ActorAlign.CENTER
        });
        titleLabel.clutter_text.selectable = true;
        mainBox.add_child(titleLabel);

        // Version Info
        let verLabel = new St.Label({
            text: `Version ${osVersion}`,
            style_class: 'pulsaros-about-subtitle',
            x_align: Clutter.ActorAlign.CENTER
        });
        verLabel.clutter_text.selectable = true;
        mainBox.add_child(verLabel);

        // Details Grid/Table
        let detailsBox = new St.BoxLayout({
            vertical: true,
            style_class: 'pulsaros-about-details'
        });
        mainBox.add_child(detailsBox);

        let addDetail = (label, value) => {
            let row = new St.BoxLayout({
                vertical: false,
                style_class: 'pulsaros-about-row'
            });
            let lbl = new St.Label({
                text: label,
                style_class: 'pulsaros-about-row-label',
                width: 140
            });
            let val = new St.Label({
                text: value,
                style_class: 'pulsaros-about-row-value'
            });
            val.clutter_text.selectable = true;
            row.add_child(lbl);
            row.add_child(val);
            detailsBox.add_child(row);
        };

        addDetail("Device Name:", hostName);
        addDetail("Processor:", cpuModel);
        addDetail("Memory:", memTotal);
        addDetail("Graphics:", gpuModel);
        addDetail("Storage:", diskInfo);

        // Copy Info Button
        this.addButton({
            label: "Copy Info",
            action: () => {
                let clipboardText = `${osName || "Pulsar OS"}\n` +
                                    `Version: ${osVersion}\n` +
                                    `Device Name: ${hostName}\n` +
                                    `Processor: ${cpuModel}\n` +
                                    `Memory: ${memTotal}\n` +
                                    `Graphics: ${gpuModel}\n` +
                                    `Storage: ${diskInfo}`;
                St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, clipboardText);
            }
        });

        // Close Button
        this.addButton({
            label: "Close",
            action: () => {
                this.close();
            },
            key: Clutter.KEY_Escape
        });
    }
});

const LockScreen = GObject.registerClass({
    GTypeName: 'PulsarosLockScreen'
}, class LockScreen extends St.Widget {
    _init(extension) {
        super._init({
            name: 'pulsaros-lockscreen',
            visible: false,
            opacity: 0,
            reactive: false,
            x_expand: true,
            y_expand: true
        });
        
        this._extension = extension;
        this._isLocked = false;
        this._hasGrab = false;
        this._authenticating = false;
        this._timerId = 0;
        this._sizeChangedId = 0;
        this._sizeChangedId2 = 0;
        this._monitorContainers = [];
        this._clocks = [];
        this._passwordEntry = null;

        this._bgSettings = new Gio.Settings({ schema_id: 'org.gnome.desktop.background' });
        this._ifaceSettings = new Gio.Settings({ schema_id: 'org.gnome.desktop.interface' });
        
        this._bgChangedId1 = this._bgSettings.connect('changed::picture-uri', () => this._updateWallpapers());
        this._bgChangedId2 = this._bgSettings.connect('changed::picture-uri-dark', () => this._updateWallpapers());
        this._bgChangedId3 = this._ifaceSettings.connect('changed::color-scheme', () => this._updateWallpapers());

        // Monitor screen and layout changes to remain fullscreen and handle multi-monitor layouts
        this._sizeChangedId = global.stage.connect('notify::width', () => this._onSizeChanged());
        this._sizeChangedId2 = global.stage.connect('notify::height', () => this._onSizeChanged());
        this._monitorsChangedId = Main.layoutManager.connect('monitors-changed', () => this._onSizeChanged());
        
        this._onSizeChanged();
    }
    
    _getWallpaperUrl() {
        try {
            let homeDir = GLib.get_home_dir();
            
            // 1. Check Pulsar OS Native Live Wallpaper configuration first
            let pulsarLiveCfg = GLib.build_filenamev([homeDir, '.config', 'pulsaros', 'live-wallpaper.json']);
            let pulsarFile = Gio.File.new_for_path(pulsarLiveCfg);
            if (pulsarFile.query_exists(null)) {
                let [ok, contents] = pulsarFile.load_contents(null);
                if (ok) {
                    let json = JSON.parse(new TextDecoder().decode(contents));
                    if (json && json.enabled && json.file) {
                        let rawPath = json.file.startsWith('file://') ? decodeURIComponent(json.file.substring(7)) : json.file;
                        let f = Gio.File.new_for_path(rawPath);
                        if (f.query_exists(null)) {
                            return f.get_uri();
                        }
                    }
                }
            }

            // 2. Check Hidamari active animated/video wallpaper configuration if present
            let hidamariPaths = [
                GLib.build_filenamev([homeDir, '.var', 'app', 'io.github.jeffshee.Hidamari', 'config', 'hidamari', 'hidamari.json']),
                GLib.build_filenamev([homeDir, '.config', 'hidamari', 'hidamari.json'])
            ];
            for (let cfgPath of hidamariPaths) {
                let file = Gio.File.new_for_path(cfgPath);
                if (file.query_exists(null)) {
                    let [ok, contents] = file.load_contents(null);
                    if (ok) {
                        let json = JSON.parse(new TextDecoder().decode(contents));
                        if (json && json.file) {
                            let rawPath = json.file.startsWith('file://') ? decodeURIComponent(json.file.substring(7)) : json.file;
                            let f = Gio.File.new_for_path(rawPath);
                            if (f.query_exists(null)) {
                                return f.get_uri();
                            }
                        }
                    }
                }
            }

            // 3. Check system default live video wallpaper
            let sddmVideo = '/var/lib/pulsar-sddm/pulsar-wallpaper.mp4';
            if (GLib.file_test(sddmVideo, GLib.FileTest.EXISTS)) {
                return `file://${sddmVideo}`;
            }

            // 2. Read active GNOME background settings
            let colorScheme = this._ifaceSettings.get_string('color-scheme');
            let uri = (colorScheme === 'prefer-dark')
                ? this._bgSettings.get_string('picture-uri-dark')
                : this._bgSettings.get_string('picture-uri');
            
            if (!uri || uri === 'none') {
                uri = this._bgSettings.get_string('picture-uri');
            }
            if (!uri || uri === 'none') {
                uri = this._bgSettings.get_string('picture-uri-dark');
            }
            if (uri && uri !== 'none') {
                // If it is an XML slideshow file, extract the real image file
                if (uri.endsWith('.xml')) {
                    let path = uri.startsWith('file://') ? uri.substring(7) : uri;
                    let xmlFile = Gio.File.new_for_path(path);
                    if (xmlFile.query_exists(null)) {
                        let [ok, contents] = xmlFile.load_contents(null);
                        if (ok) {
                            let text = new TextDecoder().decode(contents);
                            let match = text.match(/<file>([^<]+)<\/file>/);
                            if (match && match[1] && Gio.File.new_for_path(match[1]).query_exists(null)) {
                                return `file://${match[1]}`;
                            }
                        }
                    }
                }
                return uri;
            }
        } catch (e) {
            console.error("[LockScreen] Error resolving wallpaper:", e);
        }
        return `file://${this._extension.path}/background.webp`;
    }

    _getPosterUrl(videoUrl) {
        let homeDir = GLib.get_home_dir();
        let primaryPoster = GLib.build_filenamev([homeDir, '.local', 'share', 'backgrounds', 'pulsar-live-wallpaper.png']);
        if (GLib.file_test(primaryPoster, GLib.FileTest.EXISTS)) {
            return `file://${primaryPoster}`;
        }
        let sddmPoster = '/var/lib/pulsar-sddm/pulsar-wallpaper.png';
        if (GLib.file_test(sddmPoster, GLib.FileTest.EXISTS)) {
            return `file://${sddmPoster}`;
        }
        return `file:///usr/share/backgrounds/pulsar-os-tahoe.png`;
    }

    _isVideoFile(url) {
        if (!url) return false;
        let clean = url.toLowerCase();
        return clean.endsWith('.mp4') || clean.endsWith('.webm') || clean.endsWith('.mkv') || clean.endsWith('.mov') || clean.endsWith('.avi');
    }

    _startVideoWallpaper(videoPath) {
        this._stopVideoWallpaper();
        try {
            Gst.init(null);
            let localPath = videoPath.startsWith('file://') ? decodeURIComponent(videoPath.substring(7)) : videoPath;
            let file = Gio.File.new_for_path(localPath);
            if (!file.query_exists(null)) {
                return;
            }
            let videoUri = file.get_uri();
            let posterUrl = this._getPosterUrl(videoPath);

            this._videoContent = new Clutter.Image();
            for (let container of this._monitorContainers) {
                if (container._videoActor) {
                    container._videoActor.set_content(this._videoContent);
                    container._videoActor.visible = true;
                }
                container.style = `background-image: none; background-color: #000000;`;
            }

            let videoSinkBin = Gst.parse_bin_from_description(
                'videoconvert ! video/x-raw,format=RGBA ! appsink name=sink emit-signals=false max-buffers=2 drop=true sync=false',
                true
            );
            this._videoPipeline = Gst.ElementFactory.make('playbin', 'lockscreen-player');
            this._videoPipeline.set_property('uri', videoUri);
            this._videoPipeline.set_property('video-sink', videoSinkBin);
            let audioSink = Gst.ElementFactory.make('fakesink', 'lockscreen-audiosink');
            this._videoPipeline.set_property('audio-sink', audioSink);

            this._videoSink = videoSinkBin.get_by_name('sink');

            let bus = this._videoPipeline.get_bus();
            bus.add_signal_watch();
            this._busWatchId = bus.connect('message::eos', () => {
                if (this._videoPipeline) {
                    this._videoPipeline.seek_simple(Gst.Format.TIME, Gst.SeekFlags.FLUSH | Gst.SeekFlags.KEY_UNIT, 0);
                }
            });

            this._videoPipeline.set_state(Gst.State.PLAYING);

            this._videoTimerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 33, () => {
                if (!this._isLocked || !this._videoSink) {
                    return GLib.SOURCE_CONTINUE;
                }
                try {
                    let sample = this._videoSink.try_pull_sample(0);
                    if (sample) {
                        let buffer = sample.get_buffer();
                        let caps = sample.get_caps();
                        let s = caps.get_structure(0);
                        let [okW, width] = s.get_int('width');
                        let [okH, height] = s.get_int('height');
                        let [okMap, mapInfo] = buffer.map(Gst.MapFlags.READ);
                        if (okMap) {
                            let bytes = GLib.Bytes.new(mapInfo.data);
                            buffer.unmap(mapInfo);
                            if (this._videoContent) {
                                this._videoContent.set_bytes(
                                    bytes,
                                    Cogl.PixelFormat.RGBA_8888,
                                    width,
                                    height,
                                    width * 4
                                );
                            }
                        }
                    }
                } catch (pullErr) {}
                return GLib.SOURCE_CONTINUE;
            });
        } catch (e) {
            console.error("[LockScreen] Failed to start GStreamer video wallpaper:", e);
        }
    }

    _stopVideoWallpaper() {
        if (this._videoTimerId) {
            GLib.source_remove(this._videoTimerId);
            this._videoTimerId = 0;
        }
        if (this._busWatchId && this._videoPipeline) {
            let bus = this._videoPipeline.get_bus();
            bus.disconnect(this._busWatchId);
            this._busWatchId = 0;
        }
        if (this._videoPipeline) {
            this._videoPipeline.set_state(Gst.State.NULL);
            this._videoPipeline = null;
        }
        this._videoSink = null;
        this._videoContent = null;
        if (this._monitorContainers) {
            for (let container of this._monitorContainers) {
                if (container._videoActor) {
                    container._videoActor.set_content(null);
                    container._videoActor.visible = false;
                }
            }
        }
    }

    _updateWallpapers() {
        if (!this._monitorContainers || this._monitorContainers.length === 0) {
            return;
        }
        let bgUrl = this._getWallpaperUrl();
        if (this._isVideoFile(bgUrl)) {
            if (this._isLocked) {
                this._startVideoWallpaper(bgUrl);
            }
        } else {
            this._stopVideoWallpaper();
            for (let container of this._monitorContainers) {
                container.style = `background-image: url("${bgUrl}"); background-size: cover; background-position: center;`;
            }
        }
    }
    
    _onSizeChanged() {
        if (!this._isLocked) {
            this.set_position(0, 0);
            this.set_size(0, 0);
            this.visible = false;
            this.opacity = 0;
            this.reactive = false;
            return;
        }
        this.set_position(0, 0);
        this.set_size(global.stage.width, global.stage.height);
        
        this._rebuildMonitors();
    }

    _rebuildMonitors() {
        // Destroy old containers
        if (this._monitorContainers) {
            for (let container of this._monitorContainers) {
                container.destroy();
            }
        }
        this._monitorContainers = [];
        this._clocks = [];
        this._passwordEntry = null;

        this.visible = this._isLocked;
        this.opacity = this._isLocked ? 255 : 0;
        this.reactive = this._isLocked;

        let monitors = Main.layoutManager.monitors;
        let primaryMonitor = Main.layoutManager.primaryMonitor;
        let bgUrl = this._getWallpaperUrl();
        let isVideo = this._isVideoFile(bgUrl);

        for (let i = 0; i < monitors.length; i++) {
            let monitor = monitors[i];
            let isPrimary = (monitor === primaryMonitor);

            let container = new St.Widget({
                name: `pulsaros-lockscreen-monitor-${i}`,
                clip_to_allocation: true,
                reactive: true
            });

            let videoActor = new Clutter.Actor({
                name: `pulsaros-lockscreen-video-${i}`,
                x_expand: true,
                y_expand: true,
                width: monitor.width,
                height: monitor.height,
                visible: false
            });
            container.add_child(videoActor);
            container._videoActor = videoActor;

            if (this._isLocked) {
                if (isVideo) {
                    container.style = `background-image: url("${this._getPosterUrl(bgUrl)}"); background-size: cover; background-position: center;`;
                } else {
                    container.style = `background-image: url("${bgUrl}"); background-size: cover; background-position: center;`;
                }
            } else {
                container.style = 'background-image: none; background-color: transparent;';
            }
            container.set_position(monitor.x, monitor.y);
            container.set_size(monitor.width, monitor.height);

            this.add_child(container);
            this._monitorContainers.push(container);

            this._buildMonitorUI(container, monitor, isPrimary);
        }

        if (this._isLocked && isVideo) {
            this._startVideoWallpaper(bgUrl);
        }

        // Live clock updates
        if (this._isLocked) {
            this._updateClock();
        }

        if (this._isLocked) {
            GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
                if (this._passwordEntry) {
                    let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
                    activeText.grab_key_focus();
                }
                return GLib.SOURCE_REMOVE;
            });
        }
    }

    _buildMonitorUI(container, monitor, isPrimary) {
        let contentLayout = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            clip_to_allocation: true,
            x_expand: true,
            y_expand: true
        });
        contentLayout.set_size(monitor.width, monitor.height);
        container.add_child(contentLayout);

        // Click anywhere to focus password entry on primary monitor
        container.connect('button-press-event', () => {
            if (this._passwordEntry) {
                let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
                activeText.grab_key_focus();
            }
            return Clutter.EVENT_PROPAGATE;
        });

        if (isPrimary) {
            // 1. Top bar for power actions (Shutdown, Reboot)
            let topBar = new St.BoxLayout({
                style_class: 'pulsaros-lockscreen-topbar',
                x_align: Clutter.ActorAlign.END,
                y_align: Clutter.ActorAlign.START
            });
            contentLayout.add_child(topBar);

            let rebootBtn = new St.Button({
                style_class: 'pulsaros-lockscreen-power-button',
                reactive: true,
                can_focus: true,
                child: new St.Icon({
                    icon_name: 'system-restart-symbolic',
                    icon_size: 20
                })
            });
            rebootBtn.connect('clicked', () => {
                GLib.spawn_command_line_async("systemctl reboot");
            });
            topBar.add_child(rebootBtn);

            let shutdownBtn = new St.Button({
                style_class: 'pulsaros-lockscreen-power-button',
                reactive: true,
                can_focus: true,
                child: new St.Icon({
                    icon_name: 'system-shutdown-symbolic',
                    icon_size: 20
                })
            });
            shutdownBtn.connect('clicked', () => {
                GLib.spawn_command_line_async("systemctl poweroff");
            });
            topBar.add_child(shutdownBtn);

            let spacer = new St.Widget({
                style_class: 'pulsaros-lockscreen-spacer',
                height: 60
            });
            contentLayout.add_child(spacer);

            // 2. Central Clock displays
            let clockBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.START,
                style_class: 'pulsaros-lockscreen-clock-box'
            });
            contentLayout.add_child(clockBox);

            let timeLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-time-label',
                text: '00:00'
            });
            clockBox.add_child(timeLabel);

            let dateLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-date-label',
                text: ''
            });
            clockBox.add_child(dateLabel);

            this._clocks.push({ timeLabel, dateLabel });

            let middleSpacer = new St.Widget({
                y_expand: true,
                style_class: 'pulsaros-lockscreen-middle-spacer'
            });
            contentLayout.add_child(middleSpacer);

            // 3. User Credentials login card
            let userCard = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.END,
                style_class: 'pulsaros-lockscreen-user-card'
            });
            contentLayout.add_child(userCard);

            let username = GLib.get_user_name();
            let avatarWidget = new St.Widget({
                style_class: 'pulsaros-lockscreen-avatar',
                x_align: Clutter.ActorAlign.CENTER
            });

            try {
                let avatarPath = `/var/lib/AccountsService/icons/${username}`;
                let avatarFile = Gio.File.new_for_path(avatarPath);
                if (avatarFile.query_exists(null)) {
                    avatarWidget.style = `background-image: url("file://${avatarPath}"); background-size: cover; border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9);`;
                } else {
                    let faceFile = Gio.File.new_for_path(GLib.get_home_dir() + '/.face');
                    if (faceFile.query_exists(null)) {
                        avatarWidget.style = `background-image: url("file://${GLib.get_home_dir()}/.face"); background-size: cover; border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9);`;
                    } else {
                        avatarWidget.style = `border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9); background-color: rgba(255, 255, 255, 0.15);`;
                        let defaultIcon = new St.Icon({
                            icon_name: 'avatar-default-symbolic',
                            icon_size: 64,
                            style_class: 'pulsaros-lockscreen-avatar-default',
                            x_align: Clutter.ActorAlign.CENTER,
                            y_align: Clutter.ActorAlign.CENTER
                        });
                        avatarWidget.add_child(defaultIcon);
                    }
                }
            } catch (e) {
                console.error("[LockScreen] Failed to load avatar:", e);
                avatarWidget.style = `border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9); background-color: rgba(255, 255, 255, 0.15);`;
                let defaultIcon = new St.Icon({
                    icon_name: 'avatar-default-symbolic',
                    icon_size: 64,
                    style_class: 'pulsaros-lockscreen-avatar-default',
                    x_align: Clutter.ActorAlign.CENTER,
                    y_align: Clutter.ActorAlign.CENTER
                });
                avatarWidget.add_child(defaultIcon);
            }
            userCard.add_child(avatarWidget);

            let realName = username;
            try {
                let gn = GLib.get_real_name();
                if (gn && gn !== 'Unknown' && gn.trim() !== '') {
                    realName = gn;
                }
            } catch (e) {}
            realName = realName.charAt(0).toUpperCase() + realName.slice(1);

            let nameLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-name-label',
                x_align: Clutter.ActorAlign.CENTER,
                text: realName
            });
            userCard.add_child(nameLabel);

            this._passwordEntry = new St.Entry({
                style_class: 'pulsaros-lockscreen-entry',
                x_align: Clutter.ActorAlign.CENTER,
                hint_text: 'Enter Password',
                can_focus: true,
                reactive: true
            });

            let clutterText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText;
            if (!clutterText && typeof this._passwordEntry.get_clutter_text === 'function') {
                clutterText = this._passwordEntry.get_clutter_text();
            }

            if (clutterText) {
                clutterText.set_password_char('●');
                clutterText.connect('activate', () => {
                    let password = this._passwordEntry.get_text();
                    if (password && password.length > 0) {
                        this._authenticate(password);
                    }
                });
                clutterText.connect('text-changed', () => {
                    this._passwordEntry.style_class = 'pulsaros-lockscreen-entry';
                    this._passwordEntry.set_hint_text('Enter Password');
                });
                clutterText.connect('key-press-event', (actor, event) => {
                    let symbol = event.get_key_symbol();
                    if (symbol === Clutter.KEY_Escape) {
                        this._passwordEntry.set_text('');
                        return Clutter.EVENT_STOP;
                    }
                    return Clutter.EVENT_PROPAGATE;
                });
            }
            userCard.add_child(this._passwordEntry);

            let bottomSpacer = new St.Widget({
                style_class: 'pulsaros-lockscreen-bottom-spacer',
                height: 40
            });
            contentLayout.add_child(bottomSpacer);
        } else {
            // Secondary Monitor: Center a huge clock vertically and horizontally
            let topSpacer = new St.Widget({
                y_expand: true
            });
            contentLayout.add_child(topSpacer);

            let clockBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER,
                style_class: 'pulsaros-lockscreen-clock-box'
            });
            contentLayout.add_child(clockBox);

            let timeLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-time-label',
                style: 'font-size: 140px; font-weight: bold; text-shadow: 0px 4px 15px rgba(0, 0, 0, 0.3); font-family: "SF Pro Display", "SF Pro Text", "Cantarell", sans-serif; color: #ffffff; text-align: center;',
                text: '00:00'
            });
            clockBox.add_child(timeLabel);

            let dateLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-date-label',
                style: 'font-size: 28px; font-weight: 500; text-shadow: 0px 2px 10px rgba(0, 0, 0, 0.3); font-family: "SF Pro Text", "Cantarell", sans-serif; color: rgba(255, 255, 255, 0.85); text-align: center;',
                text: ''
            });
            clockBox.add_child(dateLabel);

            this._clocks.push({ timeLabel, dateLabel });

            let bottomSpacer = new St.Widget({
                y_expand: true
            });
            contentLayout.add_child(bottomSpacer);
        }
    }
    
    _updateClock() {
        let now = GLib.DateTime.new_now_local();
        let timeStr = now.format('%H:%M');
        let dateStr = now.format('%A, %B %d');
        
        if (this._clocks) {
            for (let clock of this._clocks) {
                if (clock.timeLabel) {
                    clock.timeLabel.set_text(timeStr);
                }
                if (clock.dateLabel) {
                    clock.dateLabel.set_text(dateStr);
                }
            }
        }
    }
    
    lock() {
        if (this._isLocked) return;
        this._isLocked = true;
        this.visible = true;
        this.opacity = 255;
        this.reactive = true;
        this.set_position(0, 0);
        this.set_size(global.stage.width, global.stage.height);
        
        this._rebuildMonitors();
        this._updateWallpapers();
        
        if (this._passwordEntry) {
            this._passwordEntry.text = '';
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry';
        }
        this._authenticating = false;
        
        // Put lockscreen overlay on the absolute top of the uiGroup stack
        try {
            let parent = this.get_parent();
            if (parent) {
                parent.set_child_at_index(this, -1);
            }
        } catch (e) {
            console.error("[LockScreen] Failed to raise lockscreen overlay via set_child_at_index:", e);
            try {
                Main.uiGroup.set_child_above_sibling(this, null);
            } catch (e2) {
                console.error("[LockScreen] Fallback set_child_above_sibling failed too:", e2);
            }
        }
        
        // Defer input grab and key focus to the next main loop cycle to guarantee the actor is mapped
        GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
            if (!this._isLocked) return GLib.SOURCE_REMOVE;
            
            if (Main.pushModal(this)) {
                this._hasGrab = true;
            } else {
                console.error("[LockScreen] Failed to acquire input grab");
                this._hasGrab = false;
            }
            
            if (this._passwordEntry) {
                let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
                activeText.grab_key_focus();
            }
            return GLib.SOURCE_REMOVE;
        });
        
        // Start live clock updates
        this._updateClock();
        this._timerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 1, () => {
            this._updateClock();
            return GLib.SOURCE_CONTINUE;
        });
    }
    
    unlock() {
        if (!this._isLocked) return;
        this._isLocked = false;
        this.visible = false;
        this.opacity = 0;
        this.reactive = false;
        this.set_size(0, 0);
        
        this._stopVideoWallpaper();
        
        // Destroy monitor containers when unlocked
        if (this._monitorContainers) {
            for (let container of this._monitorContainers) {
                container.destroy();
            }
        }
        this._monitorContainers = [];
        this._clocks = [];
        this._passwordEntry = null;
        
        // Release modal input grab
        if (this._hasGrab) {
            Main.popModal(this);
            this._hasGrab = false;
        }
        
        // Clean clock timer
        if (this._timerId) {
            GLib.source_remove(this._timerId);
            this._timerId = 0;
        }
    }
    
    _authenticate(password) {
        if (this._authenticating) return;
        this._authenticating = true;
        
        if (this._passwordEntry) {
            this._passwordEntry.set_reactive(false);
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry-authenticating';
        }
        
        let username = GLib.get_user_name();
        
        // Check if the PAM service file is present. If not, fallback to developer passwords for local testing on host
        let pamFile = Gio.File.new_for_path('/etc/pam.d/pulsaros-lock');
        if (!pamFile.query_exists(null)) {
            console.warn("[LockScreen] PAM service '/etc/pam.d/pulsaros-lock' is missing. Falling back to developer passwords.");
            if (password === 'pulsar' || password === 'live' || password === 'jaime') {
                this._onAuthSuccess();
            } else {
                this._onAuthFailure();
            }
            return;
        }
        
        try {
            // Run pamtester asynchronously using stdin piping
            let proc = new Gio.Subprocess({
                argv: ['/usr/bin/pamtester', 'pulsaros-lock', username, 'authenticate'],
                flags: Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
            });
            proc.init(null);
            
            let stdinStream = proc.get_stdin_pipe();
            if (stdinStream) {
                let bytes = GLib.Bytes.new(password + '\n');
                stdinStream.write_bytes(bytes, null);
                stdinStream.close(null);
            }
            
            proc.wait_async(null, (obj, res) => {
                try {
                    proc.wait_finish(res);
                    let success = proc.get_successful();
                    if (success) {
                        this._onAuthSuccess();
                    } else {
                        this._onAuthFailure();
                    }
                } catch (e) {
                    console.error("[LockScreen] pamtester wait error:", e);
                    this._onAuthFailure();
                }
            });
        } catch (e) {
            console.error("[LockScreen] pamtester launch failed:", e);
            if (password === 'pulsar' || password === 'live' || password === 'jaime') {
                this._onAuthSuccess();
            } else {
                this._onAuthFailure();
            }
        }
    }
    
    _onAuthSuccess() {
        this._authenticating = false;
        this.unlock();
    }
    
    _onAuthFailure() {
        this._authenticating = false;
        if (this._passwordEntry) {
            this._passwordEntry.set_reactive(true);
            this._passwordEntry.set_text('');
            this._passwordEntry.set_hint_text('Incorrect Password');
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry-failed';
            this._passwordEntry.grab_key_focus();
            
            // Shake animation
            let originalX = this._passwordEntry.translation_x;
            let shakeOffset = 10;
            let step = 0;
            let shakeInterval = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 50, () => {
                if (step >= 6) {
                    if (this._passwordEntry) {
                        this._passwordEntry.translation_x = originalX;
                    }
                    return GLib.SOURCE_REMOVE;
                }
                if (this._passwordEntry) {
                    this._passwordEntry.translation_x = originalX + (step % 2 === 0 ? shakeOffset : -shakeOffset);
                }
                step++;
                return GLib.SOURCE_CONTINUE;
            });
        }
    }
    
    destroy() {
        this._stopVideoWallpaper();
        if (this._bgChangedId1 && this._bgSettings) {
            this._bgSettings.disconnect(this._bgChangedId1);
            this._bgChangedId1 = 0;
        }
        if (this._bgChangedId2 && this._bgSettings) {
            this._bgSettings.disconnect(this._bgChangedId2);
            this._bgChangedId2 = 0;
        }
        if (this._bgChangedId3 && this._ifaceSettings) {
            this._ifaceSettings.disconnect(this._bgChangedId3);
            this._bgChangedId3 = 0;
        }
        if (this._monitorsChangedId) {
            Main.layoutManager.disconnect(this._monitorsChangedId);
            this._monitorsChangedId = 0;
        }
        if (this._sizeChangedId) {
            global.stage.disconnect(this._sizeChangedId);
            this._sizeChangedId = 0;
        }
        if (this._sizeChangedId2) {
            global.stage.disconnect(this._sizeChangedId2);
            this._sizeChangedId2 = 0;
        }
        if (this._timerId) {
            GLib.source_remove(this._timerId);
            this._timerId = 0;
        }
        if (this._isLocked && this._hasGrab) {
            Main.popModal(this);
        }
        if (this._monitorContainers) {
            for (let container of this._monitorContainers) {
                container.destroy();
            }
        }
        this._monitorContainers = [];
        super.destroy();
    }
});

class DesktopLiveWallpaperManager {
    constructor(extension) {
        this._extension = extension;
        this._videoPipeline = null;
        this._videoSink = null;
        this._videoTimerId = 0;
        this._busWatchId = 0;
        this._bgActor = null;
        this._videoContent = null;
        this._currentFile = null;

        this._setupConfigMonitor();
        this._updateWallpaper();
    }

    _setupConfigMonitor() {
        try {
            let homeDir = GLib.get_home_dir();
            let configDir = Gio.File.new_for_path(GLib.build_filenamev([homeDir, '.config', 'pulsaros']));
            if (!configDir.query_exists(null)) {
                configDir.make_directory_with_parents(null);
            }
            this._monitor = configDir.monitor_directory(Gio.FileMonitorFlags.NONE, null);
            this._monitorId = this._monitor.connect('changed', (mon, file, other, eventType) => {
                let name = file.get_basename();
                if (name === 'live-wallpaper.json') {
                    this._updateWallpaper();
                }
            });
        } catch (e) {
            console.error("[DesktopLiveWallpaper] Failed to setup config monitor:", e);
        }
    }

    _getConfig() {
        try {
            let homeDir = GLib.get_home_dir();
            let cfgFile = Gio.File.new_for_path(GLib.build_filenamev([homeDir, '.config', 'pulsaros', 'live-wallpaper.json']));
            if (cfgFile.query_exists(null)) {
                let [ok, contents] = cfgFile.load_contents(null);
                if (ok) {
                    return JSON.parse(new TextDecoder().decode(contents));
                }
            }
        } catch (e) {}
        return null;
    }

    _isVideoFile(path) {
        if (!path) return false;
        let clean = path.toLowerCase();
        return clean.endsWith('.mp4') || clean.endsWith('.webm') || clean.endsWith('.mkv') || clean.endsWith('.mov') || clean.endsWith('.avi');
    }

    _updateWallpaper() {
        let cfg = this._getConfig();
        if (cfg && cfg.enabled && cfg.file && (cfg.type === 'video' || this._isVideoFile(cfg.file))) {
            let localPath = cfg.file.startsWith('file://') ? decodeURIComponent(cfg.file.substring(7)) : cfg.file;
            if (GLib.file_test(localPath, GLib.FileTest.EXISTS)) {
                if (this._currentFile !== localPath) {
                    this._startVideo(localPath);
                }
                return;
            }
        }
        this._stopVideo();
    }

    _startVideo(videoPath) {
        this._stopVideo();
        this._currentFile = videoPath;
        try {
            Gst.init(null);
            let file = Gio.File.new_for_path(videoPath);
            let videoUri = file.get_uri();

            this._bgActor = new Clutter.Actor({
                name: 'pulsaros-desktop-live-wallpaper',
                x: 0,
                y: 0,
                width: global.stage.width,
                height: global.stage.height
            });

            if (Main.layoutManager._backgroundGroup) {
                Main.layoutManager._backgroundGroup.add_child(this._bgActor);
                Main.layoutManager._backgroundGroup.set_child_at_index(this._bgActor, 0);
            }

            let videoSinkBin = Gst.parse_bin_from_description(
                'videoconvert ! video/x-raw,format=RGBA ! appsink name=sink emit-signals=false max-buffers=2 drop=true sync=false',
                true
            );
            this._videoPipeline = Gst.ElementFactory.make('playbin', 'desktop-live-wallpaper');
            this._videoPipeline.set_property('uri', videoUri);
            this._videoPipeline.set_property('video-sink', videoSinkBin);
            let audioSink = Gst.ElementFactory.make('fakesink', 'desktop-audiosink');
            this._videoPipeline.set_property('audio-sink', audioSink);

            this._videoSink = videoSinkBin.get_by_name('sink');

            let bus = this._videoPipeline.get_bus();
            bus.add_signal_watch();
            this._busWatchId = bus.connect('message::eos', () => {
                if (this._videoPipeline) {
                    this._videoPipeline.seek_simple(Gst.Format.TIME, Gst.SeekFlags.FLUSH | Gst.SeekFlags.KEY_UNIT, 0);
                }
            });

            this._videoPipeline.set_state(Gst.State.PLAYING);

            this._videoTimerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 33, () => {
                if (!this._bgActor || !this._videoSink) {
                    return GLib.SOURCE_CONTINUE;
                }
                try {
                    let sample = this._videoSink.try_pull_sample(0);
                    if (sample) {
                        let buffer = sample.get_buffer();
                        let caps = sample.get_caps();
                        let s = caps.get_structure(0);
                        let [okW, width] = s.get_int('width');
                        let [okH, height] = s.get_int('height');
                        let [okMap, mapInfo] = buffer.map(Gst.MapFlags.READ);
                        if (okMap) {
                            let bytes = GLib.Bytes.new(mapInfo.data);
                            buffer.unmap(mapInfo);
                            if (!this._videoContent) {
                                this._videoContent = new Clutter.Image();
                                this._bgActor.set_content(this._videoContent);
                            }
                            this._videoContent.set_bytes(
                                bytes,
                                Cogl.PixelFormat.RGBA_8888,
                                width,
                                height,
                                width * 4
                            );
                        }
                    }
                } catch (pullErr) {}
                return GLib.SOURCE_CONTINUE;
            });
        } catch (e) {
            console.error("[DesktopLiveWallpaper] Failed to start live wallpaper:", e);
        }
    }

    _stopVideo() {
        this._currentFile = null;
        if (this._videoTimerId) {
            GLib.source_remove(this._videoTimerId);
            this._videoTimerId = 0;
        }
        if (this._busWatchId && this._videoPipeline) {
            let bus = this._videoPipeline.get_bus();
            bus.disconnect(this._busWatchId);
            this._busWatchId = 0;
        }
        if (this._videoPipeline) {
            this._videoPipeline.set_state(Gst.State.NULL);
            this._videoPipeline = null;
        }
        this._videoSink = null;
        this._videoContent = null;
        if (this._bgActor) {
            this._bgActor.destroy();
            this._bgActor = null;
        }
    }

    destroy() {
        this._stopVideo();
        if (this._monitorId && this._monitor) {
            this._monitor.disconnect(this._monitorId);
            this._monitorId = 0;
        }
    }
}

const IGNORED_APPS = [
    'welcome.py',
    'recovery.py',
    'welcome',
    'recovery',
    'pulsaros-welcome',
    'pulsaros-recovery',
    'org.pulsaros.welcome',
    'org.pulsaros.recovery',
    'calamares',
    'io.calamares.calamares',
    'gnome-control-center',
    'org.gnome.Settings',
    'spotlight-gtk',
    'io.github.jeffshee.Hidamari',
    'hidamari',
    'pulsaros-live-wallpaper',
    'ding',
    'org.rastersoft.ding',
    'ding.js',
    'desktop-icons',
    'gcr-prompter',
    'polkit-gnome-authentication-agent-1'
];

class MacOSFullscreenManager {
    constructor(extension) {
        this._extension = extension;
        this._windowSignals = new Map();
        this._spaceWindows = new Map();
        this._enabled = false;
        this._panelHidden = false;
        this._panelShowing = true;
        this._panelStrutsState = null;
        this._hideTimeoutId = 0;
        this._pointerWatcher = null;
        this._pointerWatch = null;

        this._setupSettings();
        this._setupSignals();
    }

    _hasOpenMenu() {
        try {
            if (Main.overview && Main.overview.visible) return true;
            if (Main.panel && Main.panel.menuManager && Main.panel.menuManager.activeMenu) return true;
            if (Main.panel && Main.panel.statusArea) {
                for (let k in Main.panel.statusArea) {
                    let item = Main.panel.statusArea[k];
                    if (item && item.menu && item.menu.isOpen) return true;
                }
            }
        } catch (e) {}
        return false;
    }

    _isIgnoredWindow(window) {
        if (!window) return true;
        try {
            if (window.is_override_redirect && window.is_override_redirect()) return true;
            if (window.is_skip_taskbar && window.is_skip_taskbar()) return true;
            if (window.is_on_all_workspaces && window.is_on_all_workspaces()) return true;
            let type = window.get_window_type ? window.get_window_type() : Meta.WindowType.NORMAL;
            if (type !== Meta.WindowType.NORMAL) return true;
            if (window.get_transient_for && window.get_transient_for() !== null) return true;

            let wmClass = window.get_wm_class ? (window.get_wm_class() || '') : '';
            let appId = window.get_gtk_application_id ? (window.get_gtk_application_id() || '') : '';
            let title = window.get_title ? (window.get_title() || '') : '';
            let sandboxed = window.get_sandboxed_app_id ? (window.get_sandboxed_app_id() || '') : '';

            let cmdline = '';
            let pid = window.get_pid ? window.get_pid() : 0;
            if (pid > 0) {
                try {
                    let [ok, contents] = GLib.file_get_contents(`/proc/${pid}/cmdline`);
                    if (ok) {
                        cmdline = new TextDecoder().decode(contents).replace(/\0/g, ' ');
                    }
                } catch (pe) {}
            }

            let checkStr = `${wmClass} ${appId} ${title} ${sandboxed} ${cmdline}`.toLowerCase();
            for (let ignored of IGNORED_APPS) {
                if (checkStr.includes(ignored.toLowerCase())) {
                    return true;
                }
            }
        } catch (e) {
            return true;
        }
        return false;
    }

    _setupSettings() {
        try {
            let schema = 'org.gnome.shell.extensions.pulsaros-global-menu';
            let schemaSource = Gio.SettingsSchemaSource.get_default();
            if (schemaSource && schemaSource.lookup(schema, true)) {
                this._settings = new Gio.Settings({ schema_id: schema });
                this._enabled = this._settings.get_boolean('macos-fullscreen-spaces');
                this._settings.connect('changed::macos-fullscreen-spaces', () => {
                    this._enabled = this._settings.get_boolean('macos-fullscreen-spaces');
                    this._updatePanelVisibility();
                });
            } else {
                this._enabled = false;
            }
        } catch (e) {
            this._enabled = false;
        }
    }

    _setupSignals() {
        this._windowCreatedId = global.display.connect('window-created', (display, window) => {
            this._trackWindow(window);
        });

        for (let actor of global.get_window_actors()) {
            let win = actor.meta_window;
            if (win) this._trackWindow(win);
        }

        this._setupPanelHoverTrigger();
        this._updatePanelVisibility();
    }

    _trackWindow(window) {
        if (this._isIgnoredWindow(window)) {
            return;
        }

        let winId = window.get_id ? window.get_id() : null;
        if (!winId || this._windowSignals.has(winId)) return;

        let signals = [];
        signals.push(window.connect('notify::maximized-horizontally', () => this._onMaximizeChanged(window)));
        signals.push(window.connect('notify::maximized-vertically', () => this._onMaximizeChanged(window)));
        signals.push(window.connect('notify::fullscreen', () => this._onFullscreenChanged(window)));
        signals.push(window.connect('unmanaged', () => this._untrackWindow(window)));

        this._windowSignals.set(winId, { window, signals });
    }

    _onMaximizeChanged(window) {
        if (!this._enabled || window._pulsarHandlingMaximize || this._isIgnoredWindow(window)) return;

        let isMax = window.maximized_horizontally && window.maximized_vertically;
        let isTracked = this._spaceWindows.has(window);

        if (isMax && !isTracked) {
            window._pulsarHandlingMaximize = true;
            try {
                let wsManager = global.workspace_manager;
                let activeWs = wsManager.get_active_workspace();
                let origWsIndex = activeWs.index();

                // Look for an empty workspace to the right, or append a new one
                let targetWs = null;
                let n = wsManager.n_workspaces;
                for (let i = origWsIndex + 1; i < n; i++) {
                    let ws = wsManager.get_workspace_by_index(i);
                    let nonSticky = ws.list_windows().filter(w => !w.is_on_all_workspaces() && !this._isIgnoredWindow(w));
                    if (nonSticky.length === 0) {
                        targetWs = ws;
                        break;
                    }
                }

                if (!targetWs) {
                    targetWs = wsManager.append_new_workspace(false, global.get_current_time());
                    let insertIndex = origWsIndex + 1;
                    if (wsManager.reorder_workspace && insertIndex < wsManager.n_workspaces) {
                        wsManager.reorder_workspace(targetWs, insertIndex);
                    }
                }

                window.change_workspace(targetWs);
                targetWs.activate_with_focus(window, global.get_current_time());

                this._spaceWindows.set(window, { origWsIndex });
                this._setPanelStruts(false);
                this._updatePanelVisibility();
                this._refreshSpaceWindows();
            } catch (e) {
                console.error("[MacOSFullscreen] Error moving to workspace:", e);
            } finally {
                window._pulsarHandlingMaximize = false;
            }
        } else if (!isMax && isTracked) {
            this._restoreWindow(window);
        }
    }

    _refreshSpaceWindows() {
        let wsManager = global.workspace_manager;
        let activeWs = wsManager ? wsManager.get_active_workspace() : null;
        if (!activeWs) return;

        for (let [win, data] of this._spaceWindows) {
            try {
                if (win && !win.unmanaged && win.get_workspace() === activeWs) {
                    win._pulsarHandlingMaximize = true;
                    try {
                        win.unmaximize(Meta.MaximizeFlags.BOTH);
                        win.maximize(Meta.MaximizeFlags.BOTH);
                    } finally {
                        win._pulsarHandlingMaximize = false;
                    }
                }
            } catch (e) {}
        }
    }

    _onFullscreenChanged(window) {
        this._updatePanelVisibility();
    }

    _restoreWindow(window) {
        let data = this._spaceWindows.get(window);
        if (!data) return;

        window._pulsarHandlingMaximize = true;
        try {
            let wsManager = global.workspace_manager;
            let origIndex = Math.min(data.origWsIndex, wsManager.n_workspaces - 1);
            let targetWs = wsManager.get_workspace_by_index(origIndex);

            this._spaceWindows.delete(window);

            if (window.maximized_horizontally || window.maximized_vertically) {
                window.unmaximize(Meta.MaximizeFlags.BOTH);
            }

            if (targetWs) {
                window.change_workspace(targetWs);
                targetWs.activate_with_focus(window, global.get_current_time());
            }

            this._updatePanelVisibility();
        } catch (e) {
            console.error("[MacOSFullscreen] Error restoring window:", e);
        } finally {
            window._pulsarHandlingMaximize = false;
        }
    }

    _setPanelStruts(affectsStruts) {
        if (this._panelStrutsState === affectsStruts) return;
        this._panelStrutsState = affectsStruts;
        try {
            if (Main.layoutManager && Main.layoutManager._chrome) {
                let chrome = Main.layoutManager._chrome;
                let data = chrome._findActor ? chrome._findActor(Main.layoutManager.panelBox) : null;
                if (data) {
                    data.affectsStruts = affectsStruts;
                    data.trackFullscreen = false;
                }
                if (chrome._updateRegions) {
                    chrome._updateRegions();
                } else if (Main.layoutManager._queueUpdateRegions) {
                    Main.layoutManager._queueUpdateRegions();
                }
            }
            if (Main.layoutManager.panelBox) {
                Main.layoutManager.panelBox.visible = affectsStruts;
            }
        } catch (e) {
            console.error("[MacOSFullscreen] Error setting panel struts:", e);
        }
    }

    _setupPanelHoverTrigger() {
        this._topTrigger = new Clutter.Actor({
            name: 'pulsaros-topbar-hover-trigger',
            reactive: true,
            x: 0,
            y: 0,
            width: global.stage.width || 1920,
            height: 24,
            opacity: 1,
            visible: true
        });
        Main.layoutManager.addChrome(this._topTrigger, {
            affectsStruts: false,
            trackFullscreen: false
        });

        this._topTrigger.connect('enter-event', () => {
            if (this._isCurrentWorkspaceFullscreenSpace()) {
                this._showPanel(true);
            }
        });

        this._wsChangedId = global.workspace_manager.connect('active-workspace-changed', () => {
            this._updatePanelVisibility();
        });

        this._stageResizeId = global.stage.connect('notify::width', () => {
            if (this._topTrigger) {
                this._topTrigger.set_position(0, 0);
                this._topTrigger.set_size(global.stage.width, 24);
            }
        });

        try {
            this._pointerWatcher = getPointerWatcher();
            if (this._pointerWatcher) {
                this._pointerWatch = this._pointerWatcher.addWatch(50, (x, y) => {
                    this._onPointerMoved(x, y);
                });
            }
        } catch (e) {
            console.error("[MacOSFullscreen] Error setting up pointer watcher:", e);
        }
    }

    _onPointerMoved(x, y) {
        if (!this._enabled) {
            if (this._panelHidden) this._showPanel(false);
            return;
        }
        let isSpace = this._isCurrentWorkspaceFullscreenSpace();
        if (!isSpace) {
            if (this._panelHidden) this._showPanel(false);
            return;
        }

        let panelHeight = Main.panel.height || 36;

        if (y <= 24) {
            this._showPanel(true);
        } else if (y <= panelHeight + 16) {
            if (this._hideTimeoutId) {
                GLib.source_remove(this._hideTimeoutId);
                this._hideTimeoutId = 0;
            }
        } else {
            if (this._panelShowing && !this._hasOpenMenu() && !this._hideTimeoutId) {
                this._hideTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 450, () => {
                    this._hideTimeoutId = 0;
                    if (this._isCurrentWorkspaceFullscreenSpace() && !this._hasOpenMenu()) {
                        this._hidePanel(true);
                    }
                    return GLib.SOURCE_REMOVE;
                });
            }
        }
    }

    _isCurrentWorkspaceFullscreenSpace() {
        if (!this._enabled) return false;
        let wsManager = global.workspace_manager;
        let activeWs = wsManager ? wsManager.get_active_workspace() : null;
        if (!activeWs) return false;

        // 1. Check windows tracked explicitly in _spaceWindows
        for (let [win, data] of this._spaceWindows) {
            try {
                if (win && !win.unmanaged && win.get_workspace() === activeWs) {
                    return true;
                }
            } catch (e) {}
        }

        // 2. Check if a normal non-desktop application window is fullscreen on this workspace
        let windows = activeWs.list_windows ? activeWs.list_windows() : [];
        for (let win of windows) {
            try {
                if (win && !win.unmanaged && !win.is_on_all_workspaces() && !this._isIgnoredWindow(win)) {
                    if (win.is_fullscreen && win.is_fullscreen()) {
                        return true;
                    }
                }
            } catch (e) {}
        }

        return false;
    }

    _updatePanelVisibility() {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = 0;
        }

        if (!this._enabled) {
            this._setPanelStruts(true);
            this._showPanel(false);
            if (this._topTrigger) {
                this._topTrigger.visible = false;
            }
            return;
        }

        let isSpace = this._isCurrentWorkspaceFullscreenSpace();
        if (this._topTrigger) {
            this._topTrigger.visible = isSpace;
        }

        if (isSpace) {
            this._setPanelStruts(false);
            this._hidePanel(false);
        } else {
            this._setPanelStruts(true);
            this._showPanel(false);
        }
    }

    _hidePanel(animated = false) {
        if (!this._enabled || !this._isCurrentWorkspaceFullscreenSpace()) {
            this._showPanel(false);
            return;
        }
        if (this._hasOpenMenu()) {
            return;
        }

        if (this._panelHidden && !this._panelShowing) {
            return;
        }

        this._panelHidden = true;
        this._panelShowing = false;
        let targetY = -(Main.panel.height || 36);

        this._setPanelStruts(false);
        this._refreshSpaceWindows();

        if (animated) {
            Main.panel.ease({
                translation_y: targetY,
                duration: 200,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
                onComplete: () => {
                    if (this._panelHidden && this._isCurrentWorkspaceFullscreenSpace() && Main.layoutManager.panelBox) {
                        Main.layoutManager.panelBox.visible = false;
                    }
                }
            });
        } else {
            Main.panel.remove_all_transitions();
            Main.panel.translation_y = targetY;
            if (Main.layoutManager.panelBox) {
                Main.layoutManager.panelBox.visible = false;
            }
        }
    }

    _showPanel(animated = false) {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = 0;
        }

        if (this._panelShowing && !this._panelHidden) {
            return;
        }

        this._panelHidden = false;
        this._panelShowing = true;

        if (Main.layoutManager.panelBox) {
            Main.layoutManager.panelBox.visible = true;
            Main.layoutManager.panelBox.translation_y = 0;
            if (Main.layoutManager.uiGroup && Main.layoutManager.uiGroup.set_child_above_sibling) {
                try {
                    Main.layoutManager.uiGroup.set_child_above_sibling(Main.layoutManager.panelBox, null);
                } catch (e) {}
            }
        }
        Main.panel.visible = true;
        Main.panel.reactive = true;
        Main.panel.opacity = 255;

        if (animated) {
            Main.panel.ease({
                translation_y: 0,
                duration: 200,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD
            });
        } else {
            Main.panel.remove_all_transitions();
            Main.panel.translation_y = 0;
        }
    }

    _untrackWindow(window) {
        let winId = window.get_id ? window.get_id() : null;
        if (winId && this._windowSignals.has(winId)) {
            let data = this._windowSignals.get(winId);
            for (let id of data.signals) {
                try { window.disconnect(id); } catch (e) {}
            }
            this._windowSignals.delete(winId);
        }
        this._spaceWindows.delete(window);
    }

    destroy() {
        if (this._hideTimeoutId) {
            GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = 0;
        }
        if (this._pointerWatch && this._pointerWatcher) {
            try {
                this._pointerWatcher._removeWatch(this._pointerWatch);
            } catch (e) {}
            this._pointerWatch = null;
        }
        if (this._topTrigger) {
            try {
                Main.layoutManager.removeChrome(this._topTrigger);
                this._topTrigger.destroy();
            } catch (e) {}
            this._topTrigger = null;
        }
        if (this._panelEnterId && Main.panel) {
            Main.panel.disconnect(this._panelEnterId);
            this._panelEnterId = 0;
        }
        if (this._panelLeaveId && Main.panel) {
            Main.panel.disconnect(this._panelLeaveId);
            this._panelLeaveId = 0;
        }
        if (this._windowCreatedId) {
            global.display.disconnect(this._windowCreatedId);
            this._windowCreatedId = 0;
        }
        if (this._wsChangedId) {
            global.workspace_manager.disconnect(this._wsChangedId);
            this._wsChangedId = 0;
        }
        if (this._stageResizeId) {
            global.stage.disconnect(this._stageResizeId);
            this._stageResizeId = 0;
        }
        for (let [winId, data] of this._windowSignals) {
            for (let id of data.signals) {
                try { data.window.disconnect(id); } catch (e) {}
            }
        }
        this._windowSignals.clear();
        this._spaceWindows.clear();

        try {
            if (Main.layoutManager && Main.layoutManager._chrome) {
                let chrome = Main.layoutManager._chrome;
                let data = chrome._findActor ? chrome._findActor(Main.layoutManager.panelBox) : null;
                if (data) {
                    data.affectsStruts = true;
                    data.trackFullscreen = true;
                }
                if (Main.layoutManager._queueUpdateRegions) {
                    Main.layoutManager._queueUpdateRegions();
                }
            }
            if (Main.layoutManager.panelBox) {
                Main.layoutManager.panelBox.visible = true;
            }
        } catch (e) {}

        this._showPanel(false);
    }
}

export default class PulsarosGlobalMenuExtension extends Extension {
    enable() {
        this._menuButtons = [];
        this._focusNotifyId = 0;
        this._virtualKeyboard = null;
        this._origCanLock = null;
        this._origLock = null;
        this._origUnlock = null;
        this._activeAppWindow = null;
        this._activeChangedId = 0;

        // Desktop Live Wallpaper Manager
        try {
            this._desktopLiveWallpaperManager = new DesktopLiveWallpaperManager(this);
        } catch (e) {
            console.error("[GlobalMenu] Failed to start desktop live wallpaper manager:", e);
        }

        // macOS Fullscreen Spaces & Top Panel auto-hide manager
        try {
            this._macOSFullscreenManager = new MacOSFullscreenManager(this);
        } catch (e) {
            console.error("[GlobalMenu] Failed to start macOS fullscreen manager:", e);
        }

        // Initialize the virtual keyboard device for injecting keystrokes (Wayland native)
        // Inicializar el dispositivo de teclado virtual para inyectar pulsaciones de tecla (nativo en Wayland)
        try {
            let seat = Clutter.get_default_backend().get_default_seat();
            if (seat) {
                this._virtualKeyboard = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to create virtual keyboard device:", e);
        }
        
        // Create lockscreen overlay instance and add it to top of uiGroup
        // Crear la instancia de pantalla de bloqueo y añadirla a uiGroup
        this._lockScreenOverlay = new LockScreen(this);
        Main.uiGroup.add_child(this._lockScreenOverlay);
        
        // Bind the native lock-screen media key shortcut (Super+L) to our custom LockScreen overlay
        // Vincular el atajo de teclado nativo de bloqueo (Super+L) a nuestra pantalla de bloqueo
        try {
            Main.wm.addKeybinding(
                'screensaver',
                new Gio.Settings({ schema_id: 'org.gnome.settings-daemon.plugins.media-keys' }),
                Meta.KeyBindingFlags.NONE,
                Shell.ActionMode.ALL,
                () => {
                    this._lockScreenOverlay.lock();
                }
            );
        } catch (e) {
            console.error("[GlobalMenu] Failed to bind screensaver keybinding:", e);
        }
        
        // Create each of the menu buttons
        // Crear cada uno de los botones de menú
        this._createLogoMenu();
        this._createAppMenu();
        this._createFileMenu();
        this._createEditMenu();
        this._createGoMenu();
        this._createWindowMenu();
        this._createHelpMenu();
        
        // Add menu buttons to GNOME top panel (Left Box)
        // Añadir botones de menú al panel superior de GNOME (caja izquierda)
        let pos = 1; 
        for (let button of this._menuButtons) {
            Main.panel.addToStatusArea(`global-menu-${button.uuid_suffix}`, button, pos++, 'left');
        }
        
        // Listen to window focus change notifications from display
        // Escuchar notificaciones de cambio de foco de ventana de la pantalla
        this._focusNotifyId = global.display.connect('notify::focus-window', () => {
            this._onFocusWindowChanged();
        });
        
        // Trigger initial focus state setup
        // Lanzar configuración inicial del estado de foco
        this._onFocusWindowChanged();
        
        // Force the native GNOME Shell lock button in Quick Settings to be always visible
        // Forzar a que el botón de bloqueo nativo en los Quick Settings de GNOME Shell sea siempre visible
        try {
            let actions = SystemActions?.getDefault ? SystemActions.getDefault() : null;
            if (actions) {
                this._origCanLock = Object.getOwnPropertyDescriptor(actions, 'can_lock');
                Object.defineProperty(actions, 'can_lock', {
                    get: () => true,
                    configurable: true
                });
                if (actions.notify) actions.notify('can-lock');
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to override can_lock:", e);
        }

        // Intercept native screenShield lock and unlock events to use our custom overlay
        // Interceptar eventos de bloqueo y desbloqueo nativos del screenshield para usar nuestra pantalla
        try {
            if (Main.screenShield) {
                this._origLock = Main.screenShield.lock;
                let self = this;
                Main.screenShield.lock = function(animate) {
                    try {
                        self._lockScreenOverlay.lock();
                    } catch (e) {
                        console.error("[GlobalMenu] Failed to trigger custom lock:", e);
                    }
                    return true;
                };
                
                this._origUnlock = Main.screenShield.unlock;
                Main.screenShield.unlock = function(animate) {
                    try {
                        self._lockScreenOverlay.unlock();
                    } catch (e) {
                        console.error("[GlobalMenu] Failed to trigger custom unlock:", e);
                    }
                    if (self._origUnlock) {
                        self._origUnlock.call(Main.screenShield, animate);
                    }
                };
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to override Main.screenShield:", e);
        }

        // Listen to native screenShield active status changes to unlock our overlay when the session is unlocked
        try {
            if (Main.screenShield) {
                this._activeChangedId = Main.screenShield.connect('active-changed', () => {
                    if (!Main.screenShield.active) {
                        if (this._lockScreenOverlay) {
                            this._lockScreenOverlay.unlock();
                        }
                    }
                });
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to connect to active-changed signal:", e);
        }
    }
    
    disable() {
        // Disconnect display focus notification signal
        // Desconectar la señal de notificación de foco de la pantalla
        if (this._focusNotifyId) {
            global.display.disconnect(this._focusNotifyId);
            this._focusNotifyId = 0;
        }
        
        // Unbind the screensaver keybinding
        // Desvincular el atajo de teclado de screensaver
        try {
            Main.wm.removeKeybinding('screensaver');
        } catch (e) {
            // ignore
        }
        
        // Destroy custom lockscreen overlay
        // Destruir la pantalla de bloqueo personalizada
        if (this._lockScreenOverlay) {
            Main.uiGroup.remove_child(this._lockScreenOverlay);
            this._lockScreenOverlay.destroy();
            this._lockScreenOverlay = null;
        }
        
        // Safely destroy and remove all panel buttons
        // Destruir y quitar todos los botones del panel de forma segura
        for (let button of this._menuButtons) {
            button.destroy();
        }
        this._menuButtons = [];
        
        // Restore the original can_lock descriptor
        // Restaurar el descriptor de can_lock original
        try {
            if (this._origCanLock) {
                Object.defineProperty(Shell.SystemActions.get_default(), 'can_lock', this._origCanLock);
                Shell.SystemActions.get_default().notify('can-lock');
            }
        } catch (e) {
            // ignore
        }

        // Restore original lock and unlock functions
        // Restaurar las funciones de bloqueo y desbloqueo originales
        try {
            if (Main.screenShield) {
                if (this._origLock) {
                    Main.screenShield.lock = this._origLock;
                }
                if (this._origUnlock) {
                    Main.screenShield.unlock = this._origUnlock;
                }
            }
        } catch (e) {
            // ignore
        }

        // Disconnect screenShield active-changed signal
        if (Main.screenShield && this._activeChangedId) {
            Main.screenShield.disconnect(this._activeChangedId);
            this._activeChangedId = 0;
        }

        // Clean up desktop live wallpaper manager
        if (this._desktopLiveWallpaperManager) {
            this._desktopLiveWallpaperManager.destroy();
            this._desktopLiveWallpaperManager = null;
        }

        // Clean up macOS fullscreen spaces manager
        if (this._macOSFullscreenManager) {
            this._macOSFullscreenManager.destroy();
            this._macOSFullscreenManager = null;
        }
        
        // Nullify the virtual keyboard device reference
        // Anular la referencia del dispositivo de teclado virtual
        this._virtualKeyboard = null;
    }
    
    // --- Logo Menu Button (Pulsar OS logo) ---
    // --- Botón de menú con Logo (logo de Pulsar OS) ---
    _createLogoMenu() {
        this.logoMenuButton = new GlobalMenuButton("");
        this.logoMenuButton.uuid_suffix = "logo";
        
        this.logoMenuButton.label.destroy();
        
        // Load custom Pulsar OS PNG logo icon from the extension directory path
        // Cargar el icono del logo PNG personalizado de Pulsar OS desde la ruta del directorio de la extensión
        let iconFile = Gio.File.new_for_path(this.path + '/pulsar-white-sf.png');
        let fileIcon = new Gio.FileIcon({ file: iconFile });
        this.logoMenuButton.icon = new St.Icon({
            gicon: fileIcon,
            style_class: 'global-menu-logo-icon'
        });
        this.logoMenuButton.add_child(this.logoMenuButton.icon);
        
        // About Pulsar OS
        let aboutItem = new PopupMenu.PopupMenuItem("About Pulsar OS");
        aboutItem.connect('activate', () => {
            let hostName = GLib.get_host_name();
            
            // Memory (RAM)
            let memTotal = "N/A";
            try {
                let [ok, content] = GLib.file_get_contents("/proc/meminfo");
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let match = contentStr.match(/MemTotal:\s+(\d+)\s+kB/);
                    if (match) {
                        let gb = (parseInt(match[1]) / 1024 / 1024).toFixed(1);
                        memTotal = `${gb} GB`;
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to read /proc/meminfo:", e);
            }

            // Processor (CPU)
            let cpuModel = "Unknown CPU";
            try {
                let [ok, content] = GLib.file_get_contents("/proc/cpuinfo");
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let match = contentStr.match(/model name\s+:\s+(.+)/);
                    if (match) {
                        cpuModel = match[1].trim();
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to read /proc/cpuinfo:", e);
            }

            // Graphics (GPU)
            let gpuModel = "Unknown GPU";
            try {
                let [success, stdout, stderr, status] = GLib.spawn_command_line_sync("lspci");
                if (success) {
                    let stdoutStr = new TextDecoder().decode(stdout);
                    let lines = stdoutStr.split("\n");
                    for (let line of lines) {
                        if (line.match(/VGA compatible controller|3D controller|Display controller/i)) {
                            let parts = line.split(": ");
                            if (parts.length > 1) {
                                gpuModel = parts[1].trim();
                                break;
                            }
                        }
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to run lspci:", e);
            }

            // Storage (Disk)
            let diskInfo = "N/A";
            try {
                let [success, stdout, stderr, status] = GLib.spawn_command_line_sync("df -h /");
                if (success) {
                    let stdoutStr = new TextDecoder().decode(stdout);
                    let lines = stdoutStr.split("\n");
                    if (lines.length > 1) {
                        let parts = lines[1].split(/\s+/);
                        if (parts.length > 4) {
                            let size = parts[1];
                            let avail = parts[3];
                            diskInfo = `${size} (${avail} available)`;
                        }
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to run df:", e);
            }

            // OS Name & Version from /etc/os-release
            let osName = "Pulsar OS";
            let osVersion = "rolling";
            try {
                let [ok, content] = GLib.file_get_contents("/etc/os-release");
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let prettyNameMatch = contentStr.match(/^PRETTY_NAME="?([^"\n]+)"?/m);
                    let nameMatch = contentStr.match(/^NAME="?([^"\n]+)"?/m);
                    let versionMatch = contentStr.match(/^VERSION_ID="?([^"\n]+)"?/m) ||
                                       contentStr.match(/^VERSION="?([^"\n]+)"?/m) ||
                                       contentStr.match(/^BUILD_ID="?([^"\n]+)"?/m) ||
                                       contentStr.match(/^IMAGE_VERSION="?([^"\n]+)"?/m);

                    if (prettyNameMatch) {
                        osName = prettyNameMatch[1];
                    } else if (nameMatch) {
                        osName = nameMatch[1];
                    }

                    if (versionMatch) {
                        osVersion = versionMatch[1];
                    }
                }
            } catch (e) {
                console.error("[GlobalMenu] Failed to read /etc/os-release:", e);
            }

            let dialog = new AboutDialog(osName, osVersion, hostName, cpuModel, memTotal, gpuModel, diskInfo);
            dialog.open();
        });
        this.logoMenuButton.menu.addMenuItem(aboutItem);
        
        // System Settings
        let settingsItem = new PopupMenu.PopupMenuItem("System Settings...");
        settingsItem.connect('activate', () => {
            this._runCommand("gnome-control-center");
        });
        this.logoMenuButton.menu.addMenuItem(settingsItem);
        
        // App Store
        let appStoreItem = new PopupMenu.PopupMenuItem("App Store...");
        appStoreItem.connect('activate', () => {
            this._openUri("appstream://");
        });
        this.logoMenuButton.menu.addMenuItem(appStoreItem);
        
        this.logoMenuButton.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        // Lock Screen (Solves missing lock option in system menu when not using GDM)
        // Bloquear Pantalla (Soluciona la falta de opción de bloqueo en el menú del sistema al no usar GDM)
        let lockItem = new PopupMenu.PopupMenuItem("Lock Screen");
        lockItem.connect('activate', () => {
            this._lockScreen();
        });
        this.logoMenuButton.menu.addMenuItem(lockItem);
        
        // Log Out
        let logoutItem = new PopupMenu.PopupMenuItem("Log Out...");
        logoutItem.connect('activate', () => {
            try {
                let actions = SystemActions?.getDefault ? SystemActions.getDefault() : null;
                if (actions && actions.activateLogout) {
                    actions.activateLogout();
                } else {
                    this._runCommand("gnome-session-quit --logout");
                }
            } catch (e) {
                this._runCommand("gnome-session-quit --logout");
            }
        });
        this.logoMenuButton.menu.addMenuItem(logoutItem);
        
        // Sleep
        let sleepItem = new PopupMenu.PopupMenuItem("Sleep");
        sleepItem.connect('activate', () => {
            try {
                let actions = SystemActions?.getDefault ? SystemActions.getDefault() : null;
                if (actions && actions.activateSuspend) {
                    actions.activateSuspend();
                } else {
                    this._runCommand("systemctl suspend");
                }
            } catch (e) {
                this._runCommand("systemctl suspend");
            }
        });
        this.logoMenuButton.menu.addMenuItem(sleepItem);
        
        // Restart
        let restartItem = new PopupMenu.PopupMenuItem("Restart...");
        restartItem.connect('activate', () => {
            try {
                let actions = SystemActions?.getDefault ? SystemActions.getDefault() : null;
                if (actions && actions.activateRestart) {
                    actions.activateRestart();
                } else {
                    this._runCommand("gnome-session-quit --reboot");
                }
            } catch (e) {
                this._runCommand("gnome-session-quit --reboot");
            }
        });
        this.logoMenuButton.menu.addMenuItem(restartItem);
        
        // Shut Down
        let shutdownItem = new PopupMenu.PopupMenuItem("Shut Down...");
        shutdownItem.connect('activate', () => {
            try {
                let actions = SystemActions?.getDefault ? SystemActions.getDefault() : null;
                if (actions && actions.activatePowerOff) {
                    actions.activatePowerOff();
                } else {
                    this._runCommand("gnome-session-quit --power-off");
                }
            } catch (e) {
                this._runCommand("gnome-session-quit --power-off");
            }
        });
        this.logoMenuButton.menu.addMenuItem(shutdownItem);
        
        this._menuButtons.push(this.logoMenuButton);
    }
    
    // --- App Menu Button (Finder / Active App name) ---
    // --- Botón de menú de App (Finder / Nombre de App activa) ---
    _createAppMenu() {
        this.appMenuButton = new GlobalMenuButton("Finder", true);
        this.appMenuButton.uuid_suffix = "app";
        
        // Dynamic items that will adapt their labels to the active window title
        // Elementos dinámicos que adaptarán sus etiquetas al título de la ventana activa
        this.aboutItem = new PopupMenu.PopupMenuItem("About Finder");
        this.aboutItem.connect('activate', () => {
            let activeApp = this._getAppName(this._activeAppWindow);
            Main.notify("Pulsar OS", `Active App: ${activeApp}`);
        });
        this.appMenuButton.menu.addMenuItem(this.aboutItem);
        
        this.hideItem = new PopupMenu.PopupMenuItem("Hide Finder");
        this.hideItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.minimize();
            }
        });
        this.appMenuButton.menu.addMenuItem(this.hideItem);
        
        this.quitItem = new PopupMenu.PopupMenuItem("Quit Finder");
        this.quitItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.delete(global.get_current_time());
            }
        });
        this.appMenuButton.menu.addMenuItem(this.quitItem);
        
        this._menuButtons.push(this.appMenuButton);
    }
    
    // --- File Menu Button ---
    // --- Botón de menú de Archivo ---
    _createFileMenu() {
        let fileBtn = new GlobalMenuButton("File");
        fileBtn.uuid_suffix = "file";
        
        let newWindowItem = new PopupMenu.PopupMenuItem("New Window");
        newWindowItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                let tracker = Shell.WindowTracker.get_default();
                let app = tracker.get_window_app(window);
                if (app) {
                    app.open_new_window(-1);
                    return;
                }
            }
            // If no application has focus (Finder state), launch the default file manager (Home)
            // Si ninguna aplicación tiene el foco (estado Finder), abrir gestor de archivos predeterminado (Home)
            this._openUri("file://" + GLib.get_home_dir());
        });
        fileBtn.menu.addMenuItem(newWindowItem);
        
        let closeItem = new PopupMenu.PopupMenuItem("Close Window");
        closeItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.delete(global.get_current_time());
            }
        });
        fileBtn.menu.addMenuItem(closeItem);
        
        this._menuButtons.push(fileBtn);
    }
    
    // --- Edit Menu Button ---
    // --- Botón de menú de Edición ---
    _createEditMenu() {
        let editBtn = new GlobalMenuButton("Edit");
        editBtn.uuid_suffix = "edit";
        
        let undoItem = new PopupMenu.PopupMenuItem("Undo");
        undoItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_z);
        });
        editBtn.menu.addMenuItem(undoItem);
        
        let redoItem = new PopupMenu.PopupMenuItem("Redo");
        redoItem.connect('activate', () => {
            // CTRL + Y is widely supported on Debian Gtk applications for Redo
            // CTRL + Y tiene soporte generalizado en aplicaciones Gtk de Debian para Rehacer
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_y);
        });
        editBtn.menu.addMenuItem(redoItem);
        
        editBtn.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        let cutItem = new PopupMenu.PopupMenuItem("Cut");
        cutItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_x);
        });
        editBtn.menu.addMenuItem(cutItem);
        
        let copyItem = new PopupMenu.PopupMenuItem("Copy");
        copyItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_c);
        });
        editBtn.menu.addMenuItem(copyItem);
        
        let pasteItem = new PopupMenu.PopupMenuItem("Paste");
        pasteItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Control_L], Clutter.KEY_v);
        });
        editBtn.menu.addMenuItem(pasteItem);
        
        this._menuButtons.push(editBtn);
    }
    
    // --- Go Menu Button ---
    // --- Botón de menú de Ir ---
    _createGoMenu() {
        let goBtn = new GlobalMenuButton("Go");
        goBtn.uuid_suffix = "go";
        
        let backItem = new PopupMenu.PopupMenuItem("Back");
        backItem.connect('activate', () => {
            this._sendKeyStroke([Clutter.KEY_Alt_L], Clutter.KEY_Left);
        });
        goBtn.menu.addMenuItem(backItem);
        
        goBtn.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        let homeItem = new PopupMenu.PopupMenuItem("Home");
        homeItem.connect('activate', () => {
            this._openUri("file://" + GLib.get_home_dir());
        });
        goBtn.menu.addMenuItem(homeItem);
        
        let docsItem = new PopupMenu.PopupMenuItem("Documents");
        docsItem.connect('activate', () => {
            let docsPath = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOCUMENTS) || (GLib.get_home_dir() + "/Documents");
            this._openUri("file://" + docsPath);
        });
        goBtn.menu.addMenuItem(docsItem);
        
        let downloadsItem = new PopupMenu.PopupMenuItem("Downloads");
        downloadsItem.connect('activate', () => {
            let dlPath = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOWNLOAD) || (GLib.get_home_dir() + "/Downloads");
            this._openUri("file://" + dlPath);
        });
        goBtn.menu.addMenuItem(downloadsItem);
        
        let picsItem = new PopupMenu.PopupMenuItem("Pictures");
        picsItem.connect('activate', () => {
            let picsPath = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_PICTURES) || (GLib.get_home_dir() + "/Pictures");
            this._openUri("file://" + picsPath);
        });
        goBtn.menu.addMenuItem(picsItem);
        
        this._menuButtons.push(goBtn);
    }
    
    // --- Window Menu Button ---
    // --- Botón de menú de Ventana ---
    _createWindowMenu() {
        let winBtn = new GlobalMenuButton("Window");
        winBtn.uuid_suffix = "window";
        
        let minimizeItem = new PopupMenu.PopupMenuItem("Minimize");
        minimizeItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.minimize();
            }
        });
        winBtn.menu.addMenuItem(minimizeItem);
        
        let maximizeItem = new PopupMenu.PopupMenuItem("Maximize");
        maximizeItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                if (window.get_maximized()) {
                    window.unmaximize(Meta.MaximizeFlags.BOTH);
                } else {
                    window.maximize(Meta.MaximizeFlags.BOTH);
                }
            }
        });
        winBtn.menu.addMenuItem(maximizeItem);
        
        let closeItem = new PopupMenu.PopupMenuItem("Close");
        closeItem.connect('activate', () => {
            let window = this._activeAppWindow;
            if (window) {
                window.delete(global.get_current_time());
            }
        });
        winBtn.menu.addMenuItem(closeItem);
        
        this._menuButtons.push(winBtn);
    }
    
    // --- Help Menu Button ---
    // --- Botón de menú de Ayuda ---
    _createHelpMenu() {
        let helpBtn = new GlobalMenuButton("Help");
        helpBtn.uuid_suffix = "help";
        
        // English: Link to Pulsar OS Wiki
        // Español: Enlace a la Wiki de Pulsar OS
        let wikiItem = new PopupMenu.PopupMenuItem("Pulsar OS Wiki");
        wikiItem.connect('activate', () => {
            this._openUri("https://github.com/Inled-Pulsar-OS/DOCS/wiki");
        });
        helpBtn.menu.addMenuItem(wikiItem);

        // English: Link to Pulsar OS Website
        // Español: Enlace al sitio web de Pulsar OS
        let pulsarWebItem = new PopupMenu.PopupMenuItem("Pulsar OS Website");
        pulsarWebItem.connect('activate', () => {
            this._openUri("https://os.inled.es");
        });
        helpBtn.menu.addMenuItem(pulsarWebItem);

        // English: Link to Pulsar OS Discord community
        // Español: Enlace al canal de Discord de Pulsar OS
        let discordItem = new PopupMenu.PopupMenuItem("Pulsar OS Discord");
        discordItem.connect('activate', () => {
            this._openUri("https://link.inled.es/discord");
        });
        helpBtn.menu.addMenuItem(discordItem);

        // English: Link to Pulsar OS Matrix community
        // Español: Enlace al canal de Matrix de Pulsar OS
        let matrixItem = new PopupMenu.PopupMenuItem("Pulsar OS Matrix");
        matrixItem.connect('activate', () => {
            this._openUri("https://matrix.inled.es");
        });
        helpBtn.menu.addMenuItem(matrixItem);

        // English: Separator between Pulsar OS and base system/partner links
        // Español: Separador entre los enlaces de Pulsar OS y los enlaces del sistema base/socios
        helpBtn.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        // English: Link to Inled company website
        // Español: Enlace al sitio web de la empresa Inled
        let inledItem = new PopupMenu.PopupMenuItem("Inled Website");
        inledItem.connect('activate', () => {
            this._openUri("https://inled.es");
        });
        helpBtn.menu.addMenuItem(inledItem);
        
        // English: Link to Debian base support documentation
        // Español: Enlace a la documentación de soporte base de Debian
        let debianHelpItem = new PopupMenu.PopupMenuItem("Debian Help");
        debianHelpItem.connect('activate', () => {
            this._openUri("https://www.debian.org/support");
        });
        helpBtn.menu.addMenuItem(debianHelpItem);
        
        this._menuButtons.push(helpBtn);
    }
    
    // --- Focus Event Handler ---
    // --- Manejador del Evento de Foco ---
    _onFocusWindowChanged() {
        let window = global.display.focus_window;
        
        // Check if the previously tracked window still exists
        // Comprobar si la ventana anteriormente rastreada todavía existe
        if (this._activeAppWindow) {
            let activeWindows = global.get_window_actors().map(a => a.meta_window || a.get_meta_window()).filter(Boolean);
            if (!activeWindows.includes(this._activeAppWindow)) {
                this._activeAppWindow = null;
            }
        }
        
        if (window) {
            // Check if this is a system/shell window that we want to ignore (like GNOME Shell panel or menu popups)
            // Comprobar si es una ventana del sistema o shell que queremos ignorar (como el panel o ventanas emergentes)
            let wmClass = window.get_wm_class();
            let isShell = wmClass && (wmClass.toLowerCase().includes('gnome-shell') || wmClass.toLowerCase().includes('gdm'));
            
            // In GNOME Shell, some utility windows or menus might also have no app or be system components
            if (!isShell) {
                this._activeAppWindow = window;
            }
        }
        
        let appName = this._getAppName(this._activeAppWindow);
        
        // Update panel text representation for the focused application
        // Actualizar la representación de texto en el panel para la aplicación enfocada
        this.appMenuButton.setText(appName);
        
        // Dynamically update the app menu items
        // Actualizar dinámicamente los elementos del menú de la aplicación
        this.aboutItem.label.set_text(`About ${appName}`);
        this.hideItem.label.set_text(`Hide ${appName}`);
        this.quitItem.label.set_text(`Quit ${appName}`);
        
        // Deactivate Hide/Quit if no app is open (Finder mode)
        // Desactivar Ocultar/Salir si no hay app abierta (modo Finder)
        if (appName === "Finder") {
            this.hideItem.setSensitive(false);
            this.quitItem.setSensitive(false);
        } else {
            this.hideItem.setSensitive(true);
            this.quitItem.setSensitive(true);
        }
    }
    
    // --- Helper to open URI links ---
    // --- Utilidad para abrir enlaces URI ---
    _openUri(uri) {
        try {
            Gio.AppInfo.launch_default_for_uri(uri, null);
        } catch (e) {
            console.error(`[GlobalMenu] Failed to open URI: ${uri}`, e);
        }
    }
    
    // --- Helper to lock screen ---
    // --- Utilidad para bloquear la pantalla ---
    _lockScreen() {
        try {
            if (this._lockScreenOverlay) {
                this._lockScreenOverlay.lock();
            } else if (Main.screenShield) {
                Main.screenShield.lock(true);
            } else {
                GLib.spawn_command_line_async("loginctl lock-session");
            }
        } catch (e) {
            console.error("[GlobalMenu] Failed to lock screen:", e);
            GLib.spawn_command_line_async("loginctl lock-session");
        }
    }
    
    // --- Helper to run command ---
    // --- Utilidad para ejecutar comandos ---
    _runCommand(cmd) {
        try {
            GLib.spawn_command_line_async(cmd);
        } catch (e) {
            console.error(`[GlobalMenu] Failed to run command: ${cmd}`, e);
        }
    }
    
    // --- Helper to get active application name ---
    // --- Utilidad para obtener el nombre de la app activa ---
    _getAppName(window) {
        if (!window) {
            return "Finder";
        }
        
        // Attempt to trace through window tracker app registry
        // Intentar rastrear mediante el registro de apps del WindowTracker
        let tracker = Shell.WindowTracker.get_default();
        let app = tracker.get_window_app(window);
        if (app) {
            let name = app.get_name();
            if (name) {
                return name;
            }
        }
        
        // Fallback to window WM_CLASS name (typically app-id/process type)
        // Alternativa al nombre WM_CLASS de la ventana
        let wmClass = window.get_wm_class();
        if (wmClass) {
            return wmClass.charAt(0).toUpperCase() + wmClass.slice(1);
        }
        
        // Ultimate fallback to window title
        // Última alternativa al título de la ventana
        let title = window.get_title();
        if (title) {
            return title;
        }
        
        return "Finder";
    }
    
    // --- Helper to simulate virtual keypress events ---
    // --- Utilidad para simular eventos de pulsación de teclas virtuales ---
    _sendKeyStroke(modifiers, targetKey) {
        if (!this._virtualKeyboard) {
            console.warn("[GlobalMenu] Virtual keyboard device not available");
            return;
        }
        
        let time = global.get_current_time();
        
        // 1. Press all modifiers (e.g. CTRL, ALT, SHIFT)
        // 1. Presionar todos los modificadores (p. ej., CTRL, ALT, SHIFT)
        for (let key of modifiers) {
            this._virtualKeyboard.notify_keyval(time, key, Clutter.KeyState.PRESSED);
        }
        
        // 2. Press target key
        // 2. Presionar tecla objetivo
        this._virtualKeyboard.notify_keyval(time, targetKey, Clutter.KeyState.PRESSED);
        
        // 3. Release target key
        // 3. Soltar tecla objetivo
        this._virtualKeyboard.notify_keyval(time, targetKey, Clutter.KeyState.RELEASED);
        
        // 4. Release modifiers in reverse order
        // 4. Soltar modificadores en orden inverso
        for (let i = modifiers.length - 1; i >= 0; i--) {
            this._virtualKeyboard.notify_keyval(time, modifiers[i], Clutter.KeyState.RELEASED);
        }
    }
}
