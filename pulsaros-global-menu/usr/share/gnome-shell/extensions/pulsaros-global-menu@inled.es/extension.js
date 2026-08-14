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
            text: "Pulsar OS Pear Edition",
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
                let clipboardText = `Pulsar OS Pear Edition\n` +
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

            this._videoContent = new Clutter.Image();
            for (let container of this._monitorContainers) {
                if (container._videoActor) {
                    container._videoActor.set_content(this._videoContent);
                    container._videoActor.visible = true;
                }
                container.style = 'background-image: none; background-color: #000000;';
            }

            // Construct proper custom video sink bin with GhostPad for playbin
            let bin = Gst.Bin.new('lockscreen-sinkbin');
            let conv = Gst.ElementFactory.make('videoconvert', 'conv');
            let sink = Gst.ElementFactory.make('appsink', 'sink');
            sink.set_property('emit-signals', false);
            sink.set_property('max-buffers', 2);
            sink.set_property('drop', true);
            sink.set_property('sync', false);
            let caps = Gst.Caps.from_string('video/x-raw,format=RGBA');
            sink.set_property('caps', caps);

            bin.add(conv);
            bin.add(sink);
            conv.link(sink);

            let pad = conv.get_static_pad('sink');
            let ghostpad = Gst.GhostPad.new('sink', pad);
            bin.add_pad(ghostpad);

            this._videoPipeline = Gst.ElementFactory.make('playbin', 'lockscreen-player');
            this._videoPipeline.set_property('uri', videoUri);
            this._videoPipeline.set_property('video-sink', bin);
            let audioSink = Gst.ElementFactory.make('fakesink', 'lockscreen-audiosink');
            this._videoPipeline.set_property('audio-sink', audioSink);

            this._videoSink = sink;

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
                    let sample = this._videoSink.try_pull_sample(10 * Gst.MSECOND);
                    if (sample) {
                        let buffer = sample.get_buffer();
                        let caps = sample.get_caps();
                        let s = caps.get_structure(0);
                        let [okW, width] = s.get_int('width');
                        let [okH, height] = s.get_int('height');
                        let [okMap, mapInfo] = buffer.map(Gst.MapFlags.READ);
                        if (okMap) {
                            try {
                                let ctx = Clutter.get_default_backend().get_cogl_context();
                                let format = (Cogl && Cogl.PixelFormat && Cogl.PixelFormat.RGBA_8888 !== undefined)
                                    ? Cogl.PixelFormat.RGBA_8888
                                    : 19;
                                let tex = Cogl.Texture2D.new_from_data(
                                    ctx,
                                    width,
                                    height,
                                    format,
                                    width * 4,
                                    mapInfo.data
                                );
                                if (tex) {
                                    let content = Clutter.TextureContent.new_from_texture(tex);
                                    for (let container of this._monitorContainers) {
                                        if (container._videoActor) {
                                            container._videoActor.set_content(content);
                                            container._videoActor.visible = true;
                                        }
                                    }
                                }
                            } catch (texErr) {
                                console.error("[LockScreen] Texture creation error:", texErr);
                            }
                            buffer.unmap(mapInfo);
                        }
                    }
                } catch (pullErr) {
                    console.error("[LockScreen] Video frame pull error:", pullErr);
                }
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

            // 2. Middle layout for clock and password
            let middleBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER,
                x_expand: true,
                y_expand: true,
                style_class: 'pulsaros-lockscreen-middle'
            });
            contentLayout.add_child(middleBox);

            let timeLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-time'
            });
            middleBox.add_child(timeLabel);

            let dateLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-date'
            });
            middleBox.add_child(dateLabel);

            this._clocks.push({ timeLabel, dateLabel });

            // User avatar and name
            let avatarBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                style_class: 'pulsaros-lockscreen-userbox'
            });
            middleBox.add_child(avatarBox);

            let avatarIcon = new St.Icon({
                icon_name: 'avatar-default-symbolic',
                icon_size: 72,
                style_class: 'pulsaros-lockscreen-avatar'
            });
            avatarBox.add_child(avatarIcon);

            let realName = GLib.get_real_name() || GLib.get_user_name() || 'User';
            if (realName === 'Unknown' || realName === '') {
                realName = GLib.get_user_name() || 'User';
            }
            let userLabel = new St.Label({
                text: realName,
                style_class: 'pulsaros-lockscreen-username'
            });
            avatarBox.add_child(userLabel);

            // Password entry
            let entryBox = new St.BoxLayout({
                style_class: 'pulsaros-lockscreen-entrybox',
                x_align: Clutter.ActorAlign.CENTER
            });
            middleBox.add_child(entryBox);

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
            entryBox.add_child(this._passwordEntry);

            let submitBtn = new St.Button({
                style_class: 'pulsaros-lockscreen-submit-button',
                reactive: true,
                can_focus: true,
                child: new St.Icon({
                    icon_name: 'go-next-symbolic',
                    icon_size: 16
                })
            });
            submitBtn.connect('clicked', () => {
                let password = this._passwordEntry.get_text();
                if (password && password.length > 0) {
                    this._authenticate(password);
                }
            });
            entryBox.add_child(submitBtn);
        } else {
            // Secondary monitors just show clock
            let middleBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER,
                x_expand: true,
                y_expand: true,
                style_class: 'pulsaros-lockscreen-middle'
            });
            contentLayout.add_child(middleBox);

            let timeLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-time'
            });
            middleBox.add_child(timeLabel);

            let dateLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-date'
            });
            middleBox.add_child(dateLabel);

            this._clocks.push({ timeLabel, dateLabel });
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
        
        // Use pam-auth-helper to perform secure PAM authentication
        try {
            let proc = new Gio.Subprocess({
                argv: ['/usr/lib/pulsaros/pam-auth-helper', username],
                flags: Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
            });
            
            proc.init(null);
            
            proc.communicate_utf8_async(password + '\n', null, (source, result) => {
                try {
                    let [ok, stdout, stderr] = proc.communicate_utf8_finish(result);
                    let success = proc.get_successful();
                    
                    if (success) {
                        this.unlock();
                    } else {
                        this._onAuthFailed();
                    }
                } catch (e) {
                    console.error("[LockScreen] PAM communicate error:", e);
                    this._onAuthFailed();
                }
            });
        } catch (e) {
            console.error("[LockScreen] Failed to launch pam-auth-helper:", e);
            this._onAuthFailed();
        }
    }
    
    _onAuthFailed() {
        this._authenticating = false;
        if (this._passwordEntry) {
            this._passwordEntry.set_reactive(true);
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry-error';
            this._passwordEntry.set_text('');
            this._passwordEntry.set_hint_text('Incorrect Password');
            
            let clutterText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
            clutterText.grab_key_focus();
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
        this._hideTimeoutId = 0;

        this._setupSettings();
        this._setupSignals();
    }

    _isIgnoredWindow(window) {
        if (!window) return true;
        try {
            if (window.is_override_redirect && window.is_override_redirect()) return true;
            if (window.is_skip_taskbar && window.is_skip_taskbar()) return true;
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
                    if (!this._enabled) {
                        this._showPanel(false);
                    }
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
        this._showPanel(false);
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

                let newWs = wsManager.append_new_workspace(false, global.get_current_time());
                window.change_workspace(newWs);
                window.maximize(Meta.MaximizeFlags.BOTH);
                newWs.activate_with_focus(window, global.get_current_time());

                this._spaceWindows.set(window, { origWsIndex });
                this._updatePanelVisibility();
            } catch (e) {
                console.error("[MacOSFullscreen] Error moving to workspace:", e);
            } finally {
                window._pulsarHandlingMaximize = false;
            }
        } else if (!isMax && isTracked) {
            this._restoreWindow(window);
        }
    }

    _restoreWindow(window) {
        let data = this._spaceWindows.get(window);
        if (!data) return;

        window._pulsarHandlingMaximize = true;
        try {
            let wsManager = global.workspace_manager;
            let origIndex = Math.min(data.origWsIndex, wsManager.n_workspaces - 1);
            let targetWs = wsManager.get_workspace_by_index(origIndex);

            if (window.maximized_horizontally || window.maximized_vertically) {
                window.unmaximize(Meta.MaximizeFlags.BOTH);
            }

            if (targetWs) {
                window.change_workspace(targetWs);
                targetWs.activate_with_focus(window, global.get_current_time());
            }

            this._spaceWindows.delete(window);
            this._updatePanelVisibility();
        } catch (e) {
            console.error("[MacOSFullscreen] Error restoring window:", e);
        } finally {
            window._pulsarHandlingMaximize = false;
        }
    }

    _setupPanelHoverTrigger() {
        this._topTrigger = new Clutter.Actor({
            name: 'pulsaros-topbar-hover-trigger',
            reactive: true,
            x: 0,
            y: 0,
            width: global.stage.width,
            height: 8,
            opacity: 1,
            background_color: new Clutter.Color({ red: 0, green: 0, blue: 0, alpha: 1 }),
            visible: false
        });
        Main.layoutManager.addChrome(this._topTrigger, {
            affectsInputRegion: true,
            affectsStruts: false,
            trackFullscreen: true
        });

        this._topTrigger.connect('enter-event', () => {
            if (this._isCurrentWorkspaceFullscreenSpace()) {
                this._showPanel(true);
            }
        });

        this._panelEnterId = Main.panel.connect('enter-event', () => {
            if (this._isCurrentWorkspaceFullscreenSpace()) {
                this._showPanel(true);
            }
        });

        this._panelLeaveId = Main.panel.connect('leave-event', () => {
            if (!this._isCurrentWorkspaceFullscreenSpace()) return;
            if (this._hideTimeoutId) GLib.source_remove(this._hideTimeoutId);
            this._hideTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 350, () => {
                this._hideTimeoutId = 0;
                let [x, y] = global.get_pointer();
                if (y > Main.panel.height + 5 && this._isCurrentWorkspaceFullscreenSpace()) {
                    this._hidePanel(true);
                }
                return GLib.SOURCE_REMOVE;
            });
        });

        this._wsChangedId = global.workspace_manager.connect('active-workspace-changed', () => {
            this._updatePanelVisibility();
        });

        this._stageResizeId = global.stage.connect('notify::width', () => {
            if (this._topTrigger) {
                this._topTrigger.set_size(global.stage.width, 8);
            }
        });
    }

    _isCurrentWorkspaceFullscreenSpace() {
        if (!this._enabled) return false;
        let wsManager = global.workspace_manager;
        let activeWs = wsManager.get_active_workspace();
        for (let [win, data] of this._spaceWindows) {
            try {
                if (win && !win.unmanaged && win.get_workspace() === activeWs) {
                    return true;
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

        let isSpace = this._isCurrentWorkspaceFullscreenSpace();
        if (this._topTrigger) {
            this._topTrigger.visible = isSpace;
        }

        if (isSpace) {
            this._hidePanel(true);
        } else {
            this._showPanel(false);
        }
    }

    _hidePanel(animated = false) {
        this._panelHidden = true;
        if (animated) {
            Main.panel.ease({
                translation_y: -Main.panel.height,
                opacity: 0,
                duration: 250,
                mode: Clutter.AnimationMode.EASE_OUT_CUBIC
            });
        } else {
            Main.panel.translation_y = -Main.panel.height;
            Main.panel.opacity = 0;
        }
    }

    _showPanel(animated = false) {
        this._panelHidden = false;
        Main.panel.visible = true;
        Main.panel.reactive = true;
        if (animated) {
            Main.panel.ease({
                translation_y: 0,
                opacity: 255,
                duration: 250,
                mode: Clutter.AnimationMode.EASE_OUT_CUBIC
            });
        } else {
            Main.panel.translation_y = 0;
            Main.panel.opacity = 255;
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
        if (this._topTrigger) {
            Main.layoutManager.removeChrome(this._topTrigger);
            this._topTrigger.destroy();
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
            this._origCanLock = Object.getOwnPropertyDescriptor(Shell.SystemActions.prototype, 'can_lock') || 
                                Object.getOwnPropertyDescriptor(Shell.SystemActions.get_default(), 'can_lock');
            Object.defineProperty(Shell.SystemActions.get_default(), 'can_lock', {
                get: () => true,
                configurable: true
            });
            Shell.SystemActions.get_default().notify('can-lock');
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

            // OS Name & Version
            let osName = "Pulsar OS Pear Edition";
            let osVersion = "rolling";
            try {
                let [ok, content] = GLib.file_get_contents("/etc/os-release");
                if (ok) {
                    let contentStr = new TextDecoder().decode(content);
                    let prettyNameMatch = contentStr.match(/^PRETTY_NAME="(.+)"/m);
                    if (prettyNameMatch) {
                        osName = prettyNameMatch[1].replace("Tahoe Edition", "Pear Edition");
                        if (!osName.includes("Pear Edition")) {
                            osName = osName.replace("Pulsar OS", "Pulsar OS Pear Edition");
                        }
                    }
                    let versionMatch = contentStr.match(/^VERSION_ID="(.+)"/m) || contentStr.match(/^VERSION="(.+)"/m);
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
            this._runCommand("gnome-session-quit --logout");
        });
        this.logoMenuButton.menu.addMenuItem(logoutItem);
        
        this.logoMenuButton.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        
        // Restart
        let restartItem = new PopupMenu.PopupMenuItem("Restart...");
        restartItem.connect('activate', () => {
            this._runCommand("gnome-session-quit --reboot");
        });
        this.logoMenuButton.menu.addMenuItem(restartItem);
        
        // Shut Down
        let shutdownItem = new PopupMenu.PopupMenuItem("Shut Down...");
        shutdownItem.connect('activate', () => {
            this._runCommand("gnome-session-quit --power-off");
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
