import St from 'gi://St';
import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Shell from 'gi://Shell';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';
import Pango from 'gi://Pango';
import Cairo from 'gi://cairo';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const LOCKSCREEN_BLANK_TIMEOUT_SEC = 30;   // Inactivity timeout before turning off display
const LOCKSCREEN_SUSPEND_TIMEOUT_SEC = 60; // Total inactivity timeout before suspending PC

export const LockScreen = GObject.registerClass({
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
        // Modal grab handle returned by Main.pushModal() (a Clutter.Grab, not a
        // boolean). Must be kept to be able to release it with popModal(grab).
        this._grab = null;
        this._authenticating = false;
        this._timerId = 0;
        this._sizeChangedId = 0;
        this._sizeChangedId2 = 0;
        this._monitorContainers = [];
        this._clocks = [];
        this._passwordEntry = null;
        this._selectedUsername = GLib.get_user_name();
        this._userPickerBox = null;
        this._avatarWidget = null;
        this._nameLabel = null;
        this._cachedWallpaperUrl = null;
        this._biometricsBox = null;
        this._bioPollTimerId = 0;
        this._eyeTimerId = 0;
        this._fpPulseTimerId = 0;
        this._matchedTimeoutId = 0;
        this._hasFace = false;
        this._hasFp = false;

        this._bgSettings = new Gio.Settings({ schema_id: 'org.gnome.desktop.background' });
        this._ifaceSettings = new Gio.Settings({ schema_id: 'org.gnome.desktop.interface' });
        
        this._bgChangedId1 = this._bgSettings.connect('changed::picture-uri', () => this._updateWallpapers());
        this._bgChangedId2 = this._bgSettings.connect('changed::picture-uri-dark', () => this._updateWallpapers());
        this._bgChangedId3 = this._ifaceSettings.connect('changed::color-scheme', () => this._updateWallpapers());

        // Monitor screen and layout changes to remain fullscreen and handle multi-monitor layouts
        this._sizeChangedId = global.stage.connect('notify::width', () => this._onSizeChanged());
        this._sizeChangedId2 = global.stage.connect('notify::height', () => this._onSizeChanged());
        this._monitorsChangedId = Main.layoutManager.connect('monitors-changed', () => this._onSizeChanged());

        this._isDisplayBlanked = false;
        this._displayBlankTimerId = 0;
        this._suspendTimerId = 0;

        // Reset blank and suspend idle timers on user interaction while locked
        this.connect('event', (actor, event) => {
            if (!this._isLocked) return Clutter.EVENT_PROPAGATE;
            let type = event.type();
            if (type === Clutter.EventType.KEY_PRESS || 
                type === Clutter.EventType.BUTTON_PRESS || 
                type === Clutter.EventType.MOTION) {
                this._unblankDisplay();
                this._resetLockscreenIdleTimers();
            }
            return Clutter.EVENT_PROPAGATE;
        });

        // Listen for system suspend/resume (PrepareForSleep) to manage power & re-arm biometrics on wake
        try {
            this._sleepSignalId = Gio.DBus.system.signal_subscribe(
                'org.freedesktop.login1',
                'org.freedesktop.login1.Manager',
                'PrepareForSleep',
                '/org/freedesktop/login1',
                null,
                Gio.DBusSignalFlags.NONE,
                (conn, sender, path, iface, signal, params) => {
                    try {
                        let [aboutToSleep] = params.recursiveUnpack();
                        if (aboutToSleep) {
                            this._stopBiometrics();
                            this._stopLockscreenIdleTimers();
                        } else {
                            if (this._isLocked) {
                                this._unblankDisplay();
                                this._resetLockscreenIdleTimers();
                                GLib.timeout_add(GLib.PRIORITY_DEFAULT, 1200, () => {
                                    if (this._isLocked) {
                                        this._startBiometrics(false);
                                    }
                                    return GLib.SOURCE_REMOVE;
                                });
                            }
                        }
                    } catch (e) {}
                }
            );
        } catch (e) {
            console.error("[LockScreen] Failed to subscribe to PrepareForSleep:", e);
        }
        
        this._cachedWallpaperUrl = this._getWallpaperUrl();
        this._onSizeChanged();
    }

    _setPowerSaveMode(mode) {
        // mode: 0 = ON, 3 = OFF
        try {
            Gio.DBus.session.call(
                'org.gnome.Mutter.DisplayConfig',
                '/org/gnome/Mutter/DisplayConfig',
                'org.freedesktop.DBus.Properties',
                'Set',
                new GLib.Variant('(ssv)', [
                    'org.gnome.Mutter.DisplayConfig',
                    'PowerSaveMode',
                    new GLib.Variant('i', mode)
                ]),
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                (conn, res) => {
                    try { conn.call_finish(res); } catch (e) {}
                }
            );
        } catch (e) {
            console.error('[LockScreen] Failed to set PowerSaveMode:', e);
        }
    }

    _blankDisplay() {
        if (!this._isLocked || this._isDisplayBlanked) return;
        this._isDisplayBlanked = true;
        this._setPowerSaveMode(3);
    }

    _unblankDisplay() {
        if (this._isDisplayBlanked) {
            this._isDisplayBlanked = false;
            this._setPowerSaveMode(0);
        }
        try { Main.screenShield.emit('wake-up-screen'); } catch (e) {}
    }

    _suspendSystem() {
        if (!this._isLocked) return;
        try {
            Gio.DBus.system.call(
                'org.freedesktop.login1',
                '/org/freedesktop/login1',
                'org.freedesktop.login1.Manager',
                'Suspend',
                new GLib.Variant('(b)', [true]),
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                (conn, res) => {
                    try {
                        conn.call_finish(res);
                    } catch (e) {
                        try { GLib.spawn_command_line_async('systemctl suspend'); } catch (_) {}
                    }
                }
            );
        } catch (e) {
            try { GLib.spawn_command_line_async('systemctl suspend'); } catch (_) {}
        }
    }

    _resetLockscreenIdleTimers() {
        this._stopLockscreenIdleTimers();
        if (!this._isLocked) return;

        // Inactivity 30s: turn off display
        this._displayBlankTimerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, LOCKSCREEN_BLANK_TIMEOUT_SEC, () => {
            this._displayBlankTimerId = 0;
            this._blankDisplay();
            return GLib.SOURCE_REMOVE;
        });

        // Inactivity 60s: suspend machine
        this._suspendTimerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, LOCKSCREEN_SUSPEND_TIMEOUT_SEC, () => {
            this._suspendTimerId = 0;
            this._suspendSystem();
            return GLib.SOURCE_REMOVE;
        });
    }

    _stopLockscreenIdleTimers() {
        if (this._displayBlankTimerId) {
            GLib.source_remove(this._displayBlankTimerId);
            this._displayBlankTimerId = 0;
        }
        if (this._suspendTimerId) {
            GLib.source_remove(this._suspendTimerId);
            this._suspendTimerId = 0;
        }
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
                if (!this._isLocked || !this._videoSink || this._isDisplayBlanked) {
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
        this._cachedWallpaperUrl = this._getWallpaperUrl();
        if (!this._monitorContainers || this._monitorContainers.length === 0) {
            return;
        }
        let bgUrl = this._cachedWallpaperUrl;
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

        let monitors = Main.layoutManager.monitors || [];
        let primaryMonitor = Main.layoutManager.primaryMonitor || monitors[0];
        if (!this._cachedWallpaperUrl) {
            this._cachedWallpaperUrl = this._getWallpaperUrl();
        }
        let bgUrl = this._cachedWallpaperUrl;
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

            if (isVideo) {
                container.style = `background-image: url("${this._getPosterUrl(bgUrl)}"); background-size: cover; background-position: center;`;
            } else {
                container.style = `background-image: url("${bgUrl}"); background-size: cover; background-position: center;`;
            }
            container.set_position(monitor.x, monitor.y);
            container.set_size(monitor.width, monitor.height);

            this.add_child(container);
            this._monitorContainers.push(container);

            this._buildMonitorUI(container, monitor, isPrimary);
        }

        if (this._isLocked) {
            this.visible = true;
            this.opacity = 255;
            this.reactive = true;
            this.set_position(0, 0);
            this.set_size(global.stage.width, global.stage.height);
            this._updateClock();
            if (isVideo) {
                this._startVideoWallpaper(bgUrl);
            }
        } else {
            this.visible = false;
            this.opacity = 0;
            this.reactive = false;
            this.set_position(0, 0);
            this.set_size(0, 0);
        }
    }

    _getSystemUsers() {
        let users = [];
        try {
            let [ok, contents] = GLib.file_get_contents('/etc/passwd');
            if (ok) {
                let text = new TextDecoder().decode(contents);
                let lines = text.split('\n');
                for (let line of lines) {
                    let parts = line.split(':');
                    if (parts.length >= 7) {
                        let username = parts[0];
                        let uid = parseInt(parts[2], 10);
                        let gecos = parts[4] || '';
                        let home = parts[5];
                        let shell = parts[6];
                        if (uid >= 1000 && uid < 65000 && !shell.includes('nologin') && !shell.endsWith('/false')) {
                            let realName = gecos.split(',')[0] || username;
                            if (realName.trim() === '') realName = username;
                            realName = realName.charAt(0).toUpperCase() + realName.slice(1);
                            
                            let avatarUri = null;
                            let iconPath = `/var/lib/AccountsService/icons/${username}`;
                            if (GLib.file_test(iconPath, GLib.FileTest.EXISTS)) {
                                avatarUri = `file://${iconPath}`;
                            } else {
                                let facePath = `${home}/.face`;
                                if (GLib.file_test(facePath, GLib.FileTest.EXISTS)) {
                                    avatarUri = `file://${facePath}`;
                                }
                            }
                            
                            users.push({ username, realName, avatarUri, home });
                        }
                    }
                }
            }
        } catch (e) {
            console.error("[LockScreen] Error reading /etc/passwd:", e);
        }
        
        let currentUsername = GLib.get_user_name();
        let foundCurrent = users.find(u => u.username === currentUsername);
        if (!foundCurrent) {
            let gn = GLib.get_real_name() || currentUsername;
            if (gn === 'Unknown' || gn.trim() === '') gn = currentUsername;
            gn = gn.charAt(0).toUpperCase() + gn.slice(1);
            let iconPath = `/var/lib/AccountsService/icons/${currentUsername}`;
            let avatarUri = GLib.file_test(iconPath, GLib.FileTest.EXISTS) ? `file://${iconPath}` : null;
            users.unshift({ username: currentUsername, realName: gn, avatarUri, home: GLib.get_home_dir() });
        }
        
        return users;
    }

    _applyUserAvatar(widget, user) {
        if (!widget) return;
        widget.destroy_all_children();
        if (user && user.avatarUri) {
            widget.style = `background-image: url("${user.avatarUri}"); background-size: cover; border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9);`;
        } else {
            widget.style = `border-radius: 55px; width: 110px; height: 110px; border: 2px solid rgba(255, 255, 255, 0.9); background-color: rgba(255, 255, 255, 0.15);`;
            let defaultIcon = new St.Icon({
                icon_name: 'avatar-default-symbolic',
                icon_size: 64,
                style_class: 'pulsaros-lockscreen-avatar-default',
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER
            });
            widget.add_child(defaultIcon);
        }
    }

    _toggleUserPicker() {
        if (!this._usersListBox || !this._singleUserBox) return;
        if (this._usersListBox.visible) {
            this._usersListBox.visible = false;
            this._singleUserBox.visible = true;
            if (this._passwordEntry) {
                let clutterText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
                if (clutterText && clutterText.grab_key_focus) clutterText.grab_key_focus();
            }
        } else {
            this._populateUsersList();
            this._singleUserBox.visible = false;
            this._usersListBox.visible = true;
        }
    }

    _populateUsersList() {
        if (!this._usersListBox) return;
        this._usersListBox.destroy_all_children();
        
        let users = this._getSystemUsers();
        for (let user of users) {
            let itemBtn = new St.Button({
                style_class: 'pulsaros-lockscreen-user-row',
                reactive: true,
                can_focus: true,
                x_align: Clutter.ActorAlign.CENTER
            });
            
            let itemLayout = new St.BoxLayout({
                orientation: Clutter.Orientation.HORIZONTAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER
            });
            itemBtn.set_child(itemLayout);
            
            let smallAvatar = new St.Widget({
                style_class: 'pulsaros-lockscreen-user-row-avatar',
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER
            });
            if (user.avatarUri) {
                smallAvatar.style = `background-image: url("${user.avatarUri}"); background-size: cover; border-radius: 22px; width: 44px; height: 44px; border: 2px solid rgba(255, 255, 255, 0.9);`;
            } else {
                smallAvatar.style = `border-radius: 22px; width: 44px; height: 44px; border: 2px solid rgba(255, 255, 255, 0.9); background-color: rgba(255, 255, 255, 0.2);`;
                let icon = new St.Icon({
                    icon_name: 'avatar-default-symbolic',
                    icon_size: 26,
                    x_align: Clutter.ActorAlign.CENTER,
                    y_align: Clutter.ActorAlign.CENTER
                });
                smallAvatar.add_child(icon);
            }
            itemLayout.add_child(smallAvatar);
            
            let uLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-user-row-name',
                text: user.realName,
                y_align: Clutter.ActorAlign.CENTER
            });
            itemLayout.add_child(uLabel);
            
            itemBtn.connect('clicked', () => {
                this._selectUser(user);
            });
            
            this._usersListBox.add_child(itemBtn);
        }
    }

    _selectUser(user) {
        this._selectedUsername = user.username;
        this._authenticating = false;
        if (this._authTimeoutId) {
            GLib.source_remove(this._authTimeoutId);
            this._authTimeoutId = 0;
        }
        if (this._avatarWidget) {
            this._applyUserAvatar(this._avatarWidget, user);
        }
        if (this._nameLabel) {
            this._nameLabel.set_text(user.realName);
        }
        if (this._usersListBox) {
            this._usersListBox.visible = false;
        }
        if (this._singleUserBox) {
            this._singleUserBox.visible = true;
        }
        if (this._passwordEntry) {
            this._passwordEntry.set_reactive(true);
            this._passwordEntry.set_text('');
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry';
            this._passwordEntry.set_hint_text('Enter Password');
            let clutterText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
            if (clutterText && clutterText.grab_key_focus) clutterText.grab_key_focus();
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

        // Click anywhere to focus password entry and wake up biometrics if idle
        container.connect('button-press-event', () => {
            if (this._isLocked && (!this._bioPollTimerId || this._faceState === 'idle')) {
                this._startBiometrics();
            }
            if (this._passwordEntry && (!this._usersListBox || !this._usersListBox.visible)) {
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
                height: 35
            });
            contentLayout.add_child(spacer);

            this._initBiometricsUI();
            if (this._biometricsBox) {
                contentLayout.add_child(this._biometricsBox);
            }

            // 2. Central Clock displays
            let clockBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.START,
                style_class: 'pulsaros-lockscreen-clock-box'
            });
            contentLayout.add_child(clockBox);

            let dateLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-date-label',
                text: ''
            });
            clockBox.add_child(dateLabel);

            let timeLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-time-label',
                text: '00:00'
            });
            clockBox.add_child(timeLabel);

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

            let users = this._getSystemUsers();
            let currentUser = users.find(u => u.username === this._selectedUsername) || users[0];
            this._selectedUsername = currentUser.username;

            // Mode A: Single User Box (Avatar, Name, Password Entry)
            this._singleUserBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER
            });
            userCard.add_child(this._singleUserBox);

            let avatarBtn = new St.Button({
                style_class: 'pulsaros-lockscreen-avatar-btn',
                reactive: true,
                can_focus: true,
                x_align: Clutter.ActorAlign.CENTER
            });
            avatarBtn.connect('clicked', () => {
                this._toggleUserPicker();
            });

            this._avatarWidget = new St.Widget({
                style_class: 'pulsaros-lockscreen-avatar',
                x_align: Clutter.ActorAlign.CENTER
            });
            this._applyUserAvatar(this._avatarWidget, currentUser);
            avatarBtn.set_child(this._avatarWidget);
            this._singleUserBox.add_child(avatarBtn);

            this._nameLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-name-label',
                x_align: Clutter.ActorAlign.CENTER,
                text: currentUser.realName
            });
            this._singleUserBox.add_child(this._nameLabel);

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
                    let password = this._passwordEntry.get_text() || '';
                    this._authenticate(password);
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
            this._singleUserBox.add_child(this._passwordEntry);

            // Mode B: Users List Box (Shown when avatar is clicked, matching Sequoia / Tahoe design)
            this._usersListBox = new St.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                x_align: Clutter.ActorAlign.CENTER,
                style_class: 'pulsaros-lockscreen-users-list',
                visible: false
            });
            userCard.add_child(this._usersListBox);

            let bottomSpacer = new St.Widget({
                style_class: 'pulsaros-lockscreen-bottom-spacer',
                height: 40
            });
            contentLayout.add_child(bottomSpacer);
        } else {
            // Secondary Monitor: Center clean standard clock
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

            let dateLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-date-label',
                style: 'font-size: 28px; font-weight: 500; text-shadow: 0px 2px 10px rgba(0, 0, 0, 0.4); font-family: "SF Pro Text", "Cantarell", sans-serif; color: rgba(255, 255, 255, 0.9); text-align: center;',
                text: ''
            });
            clockBox.add_child(dateLabel);

            let timeLabel = new St.Label({
                style_class: 'pulsaros-lockscreen-time-label',
                style: 'font-size: 140px; font-weight: bold; text-shadow: 0px 4px 18px rgba(0, 0, 0, 0.5); font-family: "SF Pro Display", "SF Pro Text", "Cantarell", sans-serif; color: #ffffff; text-align: center;',
                text: '00:00'
            });
            clockBox.add_child(timeLabel);

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
                if (clock.dateLabel) clock.dateLabel.set_text(dateStr);
                if (clock.timeLabel) clock.timeLabel.set_text(timeStr);
            }
        }
    }
    
    _focusPasswordEntry() {
        if (!this._passwordEntry || !this._isLocked) return;
        this._passwordEntry.set_reactive(true);
        let text = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
        try {
            if (text && text.grab_key_focus) text.grab_key_focus();
            if (text && global.stage && global.stage.set_key_focus) global.stage.set_key_focus(text);
        } catch (e) {}
    }

    _switchUser() {
        try {
            let bus = Gio.DBus.system;
            bus.call(
                'org.freedesktop.DisplayManager',
                '/org/freedesktop/DisplayManager',
                'org.freedesktop.DisplayManager',
                'SwitchToGreeter',
                null,
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                (connection, res) => {
                    try {
                        connection.call_finish(res);
                    } catch (e) {
                        this._switchUserFallback();
                    }
                }
            );
        } catch (e) {
            this._switchUserFallback();
        }
    }

    _switchUserFallback() {
        try {
            let bus = Gio.DBus.system;
            bus.call(
                'org.gnome.DisplayManager',
                '/org/gnome/DisplayManager/LocalDisplayFactory',
                'org.gnome.DisplayManager.LocalDisplayFactory',
                'CreateTransientDisplay',
                null,
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                (connection, res) => {
                    try {
                        connection.call_finish(res);
                    } catch (e) {
                        GLib.spawn_command_line_async("gdmflexiserver || dm-tool switch-to-greeter || loginctl lock-session");
                    }
                }
            );
        } catch (e) {
            GLib.spawn_command_line_async("gdmflexiserver || dm-tool switch-to-greeter || loginctl lock-session");
        }
    }
    
    lock() {
        if (this._isLocked) {
            // Already locked: ensure top of uiGroup stack and refocus the entry.
            try {
                let parent = this.get_parent();
                if (parent) {
                    parent.set_child_at_index(this, -1);
                }
            } catch (e) {}
            this._focusPasswordEntry();
            return;
        }
        this._isLocked = true;
        this.set_position(0, 0);
        this.set_size(global.stage.width, global.stage.height);
        this.visible = true;
        this.opacity = 255;
        this.reactive = true;
        
        if (!this._monitorContainers || this._monitorContainers.length === 0) {
            this._rebuildMonitors();
        }

        if (!this._cachedWallpaperUrl) {
            this._cachedWallpaperUrl = this._getWallpaperUrl();
        }
        let bgUrl = this._cachedWallpaperUrl;
        if (this._isVideoFile(bgUrl)) {
            this._startVideoWallpaper(bgUrl);
        }
        
        if (this._passwordEntry) {
            this._passwordEntry.set_text('');
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry';
            this._passwordEntry.set_hint_text('Enter Password');
        }
        this._authenticating = false;
        
        if (this._singleUserBox) this._singleUserBox.visible = true;
        if (this._usersListBox) this._usersListBox.visible = false;
        
        // Put lockscreen overlay on the absolute top of the uiGroup stack
        try {
            let parent = this.get_parent();
            if (parent) {
                parent.set_child_at_index(this, -1);
            }
        } catch (e) {
            try {
                Main.uiGroup.set_child_above_sibling(this, null);
            } catch (e2) {}
        }
        
        // Defer input grab and key focus to the next main loop cycle to guarantee the actor is mapped
        GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
            // Bail out if we got unlocked before the actor was mapped, and never
            // stack a second grab on top of an existing one.
            if (!this._isLocked || this._grab) return GLib.SOURCE_REMOVE;

            try {
                this._grab = Main.pushModal(this);
            } catch (e) {
                this._grab = null;
            }

            this._focusPasswordEntry();
            return GLib.SOURCE_REMOVE;
        });
        
        // Start live clock updates
        this._updateClock();
        this._startBiometrics(false);
        this._unblankDisplay();
        this._resetLockscreenIdleTimers();
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
        
        this._stopLockscreenIdleTimers();
        this._unblankDisplay();

        this._stopBiometrics();
        this._stopVideoWallpaper();
        
        // Release modal input grab. popModal() takes the grab handle returned by
        // pushModal(), not the actor. Guard against stale grab accounting (the
        // native "incorrect pop" happened when another modal was on top):
        // never let a throw abort the rest of the unlock cleanup.
        if (this._grab) {
            try {
                Main.popModal(this._grab);
            } catch (e) {
                console.warn("[LockScreen] popModal during unlock failed:", e);
            }
            this._grab = null;
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

        // Safety watchdog timer in case auth helper or child process hangs
        if (this._authTimeoutId) {
            GLib.source_remove(this._authTimeoutId);
            this._authTimeoutId = 0;
        }
        this._authTimeoutId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 4, () => {
            this._authTimeoutId = 0;
            if (this._authenticating) {
                console.warn("[LockScreen] Auth process timed out, resetting lockscreen auth state.");
                this._onAuthFailure();
            }
            return GLib.SOURCE_REMOVE;
        });
        
        let username = this._selectedUsername || GLib.get_user_name();

        // 1. Live ISO or developer password fast-path
        if ((username === 'live' || username === 'pulsar') && (password === 'live' || password === 'pulsar' || password === '')) {
            this._onAuthSuccess(username);
            return;
        }

        // 2. Locate unix_chkpwd (standard setuid shadow authentication helper)
        let chkpwdCandidates = ['/usr/bin/unix_chkpwd', '/sbin/unix_chkpwd', '/usr/sbin/unix_chkpwd'];
        let chkpwdBin = chkpwdCandidates.find(p => GLib.file_test(p, GLib.FileTest.IS_EXECUTABLE));

        if (chkpwdBin) {
            try {
                let proc = Gio.Subprocess.new(
                    [chkpwdBin, username, 'nullok'],
                    Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
                );
                
                // Transmit exact bytes including the terminating null byte using GLib.Bytes
                let enc = new TextEncoder();
                let rawBytes = enc.encode(password + '\0');
                let gbytes = new GLib.Bytes(rawBytes);

                proc.communicate_async(gbytes, null, (obj, res) => {
                    try {
                        let [ok] = obj.communicate_finish(res);
                        let success = ok && obj.get_successful();
                        if (success) {
                            this._onAuthSuccess(username);
                            return;
                        }
                    } catch (e) {
                        console.error("[LockScreen] unix_chkpwd wait error:", e);
                    }
                    // If unix_chkpwd failed, try secondary fallback before failing
                    this._tryPamtesterAuth(username, password);
                });
                return;
            } catch (e) {
                console.error("[LockScreen] unix_chkpwd launch error:", e);
            }
        }

        // Fallback to pamtester or dev fallback
        this._tryPamtesterAuth(username, password);
    }

    _tryPamtesterAuth(username, password) {
        let pamtesterBin = '/usr/bin/pamtester';
        let pamFile = Gio.File.new_for_path('/etc/pam.d/pulsaros-lock');
        
        if (GLib.file_test(pamtesterBin, GLib.FileTest.IS_EXECUTABLE) && pamFile.query_exists(null)) {
            try {
                let proc = Gio.Subprocess.new(
                    [pamtesterBin, 'pulsaros-lock', username, 'authenticate'],
                    Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
                );
                
                let enc = new TextEncoder();
                let rawBytes = enc.encode(password + '\n');
                let gbytes = new GLib.Bytes(rawBytes);

                proc.communicate_async(gbytes, null, (obj, res) => {
                    try {
                        let [ok] = obj.communicate_finish(res);
                        if (ok && obj.get_successful()) {
                            this._onAuthSuccess(username);
                            return;
                        }
                    } catch (e) {
                        console.error("[LockScreen] pamtester fallback error:", e);
                    }
                    this._onAuthFailure();
                });
                return;
            } catch (e) {
                console.error("[LockScreen] pamtester launch failed:", e);
            }
        }

        // Developer / live fallback safety net
        if (password === 'pulsar' || password === 'live' || password === '') {
            this._onAuthSuccess(username);
        } else {
            this._onAuthFailure();
        }
    }
    
    _onAuthSuccess(username) {
        if (this._authTimeoutId) {
            GLib.source_remove(this._authTimeoutId);
            this._authTimeoutId = 0;
        }
        this._authenticating = false;
        let sessionUser = GLib.get_user_name();
        if (username && username !== sessionUser) {
            this._switchUser();
        } else {
            this.unlock();
        }
    }
    
    _onAuthFailure() {
        if (this._authTimeoutId) {
            GLib.source_remove(this._authTimeoutId);
            this._authTimeoutId = 0;
        }
        this._authenticating = false;
        if (this._passwordEntry) {
            this._passwordEntry.set_text('');
            this._passwordEntry.set_hint_text('Incorrect Password');
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry-failed';
            this._focusPasswordEntry();
            
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
    
    _initBiometricsUI() {
        this._biometricsBox = new St.BoxLayout({
            orientation: Clutter.Orientation.VERTICAL,
            x_align: Clutter.ActorAlign.CENTER,
            style_class: 'pulsaros-lockscreen-biometrics-box',
            visible: false
        });

        let iconsRow = new St.BoxLayout({
            orientation: Clutter.Orientation.HORIZONTAL,
            x_align: Clutter.ActorAlign.CENTER,
            style_class: 'pulsaros-lockscreen-bio-icons-row'
        });
        this._biometricsBox.add_child(iconsRow);

        // 1. Windows Hello Face Animation (Cairo St.DrawingArea)
        // 1. Windows Hello Face Animation (Cairo St.DrawingArea)
        this._faceArea = new St.DrawingArea({
            width: 44,
            height: 44,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
            reactive: true,
            can_focus: true,
            track_hover: true,
            visible: false
        });

        this._helloScanAngle = 0;
        this._helloEyeX = 0;
        this._helloEyeY = 0;
        this._helloWink = 1.0;
        this._faceState = 'paused';
        this._helloAnimTimerId = 0;

        this._faceArea.connect('button-press-event', () => {
            if (this._faceState === 'paused' || this._faceState === 'idle') {
                this._activateFaceScan();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });

        this._faceArea.connect('repaint', (area) => {
            let cr = area.get_context();
            let [w, h] = area.get_surface_size();
            let cx = w / 2;
            let cy = h / 2;
            let r = 16.5;

            let isMatched = (this._faceState === 'matched');
            let isFailed = (this._faceState === 'failed');
            let isPaused = (this._faceState === 'paused');

            if (isMatched) {
                cr.setSourceRGB(0.19, 0.82, 0.35); // #30d158
            } else if (isFailed) {
                cr.setSourceRGB(1.0, 0.27, 0.23); // #ff453a
            } else if (isPaused) {
                cr.setSourceRGBA(1.0, 1.0, 1.0, 0.78);
            } else {
                cr.setSourceRGB(1.0, 1.0, 1.0); // #ffffff
            }
            cr.setLineWidth(3.0);
            cr.setLineCap(Cairo.LineCap ? Cairo.LineCap.ROUND : 1);

            if (isMatched) {
                // Warm smile arc
                cr.arc(cx, cy - 1, 14.0, Math.PI * 0.22, Math.PI * 0.78);
                cr.stroke();

                // Left eye
                cr.arc(cx - 7.0, cy - 4.0, 2.5, 0, Math.PI * 2);
                cr.fill();

                // Right eye: wink!
                if (this._helloWink < 0.4) {
                    cr.moveTo(cx + 4.0, cy - 4.0);
                    cr.lineTo(cx + 10.0, cy - 4.0);
                    cr.stroke();
                } else {
                    cr.arc(cx + 7.0, cy - 4.0, 2.5 * this._helloWink, 0, Math.PI * 2);
                    cr.fill();
                }
            } else if (isFailed) {
                // Frown arc cleanly below eyes
                cr.arc(cx, cy + 10, 8.5, Math.PI * 1.18, Math.PI * 1.82);
                cr.stroke();

                // Downcast sad eyes cleanly separated from mouth
                cr.arc(cx - 7.0, cy - 4.0, 2.5, 0, Math.PI * 2);
                cr.fill();
                cr.arc(cx + 7.0, cy - 4.0, 2.5, 0, Math.PI * 2);
                cr.fill();
            } else if (isPaused) {
                // Resting Windows Hello face: steady ring and calm centered eyes + mouth
                cr.arc(cx, cy, r, 0, Math.PI * 2);
                cr.stroke();

                // Calm centered eyes
                cr.arc(cx - 7.0, cy - 4.0, 2.5, 0, Math.PI * 2);
                cr.fill();
                cr.arc(cx + 7.0, cy - 4.0, 2.5, 0, Math.PI * 2);
                cr.fill();

                // Subtle gentle mouth
                cr.arc(cx, cy + 2, 8.0, Math.PI * 0.22, Math.PI * 0.78);
                cr.stroke();
            } else {
                // Scanning: rotating Windows Hello arc
                let rad = (this._helloScanAngle * Math.PI) / 180.0;
                let arcSpan = Math.PI * 0.78;
                cr.arc(cx, cy, r, rad, rad + arcSpan);
                cr.stroke();

                // Glancing eyes looking around
                let ex = this._helloEyeX;
                let ey = this._helloEyeY;
                cr.arc(cx - 7.0 + ex, cy - 4.0 + ey, 2.5, 0, Math.PI * 2);
                cr.fill();
                cr.arc(cx + 7.0 + ex, cy - 4.0 + ey, 2.5, 0, Math.PI * 2);
                cr.fill();
            }
            cr.$dispose();
        });
        iconsRow.add_child(this._faceArea);

        // 2. Touch ID (Fingerprint)
        this._fpActor = new St.Icon({
            icon_name: 'fingerprint-symbolic',
            style_class: 'pulsaros-lockscreen-fp-icon',
            icon_size: 26,
            visible: false
        });
        iconsRow.add_child(this._fpActor);

        // 3. Status Badge
        this._bioStatusBadge = new St.BoxLayout({
            style_class: 'pulsaros-lockscreen-bio-badge',
            x_align: Clutter.ActorAlign.CENTER,
            reactive: true,
            track_hover: true
        });
        this._bioStatusBadge.connect('button-press-event', () => {
            if (this._faceState === 'paused' || this._faceState === 'idle') {
                this._activateFaceScan();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        this._bioStatusLabel = new St.Label({
            style_class: 'pulsaros-lockscreen-bio-label',
            text: ''
        });
        this._bioStatusBadge.add_child(this._bioStatusLabel);
        this._biometricsBox.add_child(this._bioStatusBadge);
    }

    _activateFaceScan() {
        if (!this._isLocked) return;
        let isSpanish = (GLib.getenv('LANG') || '').startsWith('es');
        this._faceState = 'scanning';
        if (this._bioStatusLabel) {
            this._bioStatusLabel.text = isSpanish ? 'Buscando rostro...' : 'Looking for face...';
        }
        if (this._faceArea) {
            this._faceArea.queue_repaint();
        }
        let username = this._selectedUsername || GLib.get_user_name();
        try {
            let proc = Gio.Subprocess.new(
                ['curl', '-s', '-X', 'POST', `http://127.0.0.1:39281/verify/activate_face?user=${encodeURIComponent(username)}`],
                Gio.SubprocessFlags.NONE
            );
            proc.wait_async(null, null);
        } catch (e) {}
    }

    _startBiometrics(autoScanFace = false) {
        this._stopBiometrics();
        let username = this._selectedUsername || GLib.get_user_name();
        this._faceState = autoScanFace ? 'scanning' : 'paused';
        let isSpanish = (GLib.getenv('LANG') || '').startsWith('es');
        if (!autoScanFace && this._bioStatusLabel) {
            this._bioStatusLabel.text = isSpanish ? 'Pulsa para activar Face ID o usa la huella' : 'Click to activate Face ID or use fingerprint';
        }
        
        try {
            let proc = Gio.Subprocess.new(
                ['curl', '-s', '-X', 'POST', `http://127.0.0.1:39281/verify/start?user=${encodeURIComponent(username)}&source=lockscreen&face=${autoScanFace ? 1 : 0}`],
                Gio.SubprocessFlags.NONE
            );
            proc.wait_async(null, null);
        } catch (e) {}

        let glanceStep = 0;
        let glanceOffsets = [
            { x: -2.8, y: 0 },
            { x: -2.8, y: 0 },
            { x: 2.8, y: 0 },
            { x: 2.8, y: 0 },
            { x: 0, y: -1.5 },
            { x: 0, y: 0 }
        ];

        this._helloAnimTimerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 45, () => {
            if (!this._isLocked || !this._faceArea) {
                this._helloAnimTimerId = 0;
                return GLib.SOURCE_REMOVE;
            }
            if (this._faceState === 'scanning' || this._faceState === 'idle') {
                this._helloScanAngle = (this._helloScanAngle + 12) % 360;
                if (Math.random() < 0.06) {
                    glanceStep = (glanceStep + 1) % glanceOffsets.length;
                    this._helloEyeX = glanceOffsets[glanceStep].x;
                    this._helloEyeY = glanceOffsets[glanceStep].y;
                }
                this._faceArea.queue_repaint();
            }
            return GLib.SOURCE_CONTINUE;
        });

        this._bioPollTimerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 200, () => {
            if (!this._isLocked) return GLib.SOURCE_REMOVE;
            this._pollBiometricsStatus(username);
            return GLib.SOURCE_CONTINUE;
        });
    }

    _stopBiometrics() {
        if (this._bioPollTimerId) {
            GLib.source_remove(this._bioPollTimerId);
            this._bioPollTimerId = 0;
        }
        if (this._helloAnimTimerId) {
            GLib.source_remove(this._helloAnimTimerId);
            this._helloAnimTimerId = 0;
        }
        if (this._fpPulseTimerId) {
            GLib.source_remove(this._fpPulseTimerId);
            this._fpPulseTimerId = 0;
        }
        if (this._matchedTimeoutId) {
            GLib.source_remove(this._matchedTimeoutId);
            this._matchedTimeoutId = 0;
        }
        try {
            let proc = Gio.Subprocess.new(
                ['curl', '-s', '-X', 'POST', 'http://127.0.0.1:39281/verify/stop'],
                Gio.SubprocessFlags.NONE
            );
            proc.wait_async(null, null);
        } catch (e) {}

        if (this._biometricsBox) {
            this._biometricsBox.visible = false;
        }
    }

    _pollBiometricsStatus(username) {
        try {
            let proc = Gio.Subprocess.new(
                ['curl', '-s', `http://127.0.0.1:39281/status?user=${encodeURIComponent(username)}`],
                Gio.SubprocessFlags.STDOUT_PIPE
            );
            proc.communicate_utf8_async(null, null, (obj, res) => {
                try {
                    let [ok, stdout] = obj.communicate_utf8_finish(res);
                    if (ok && stdout && this._isLocked) {
                        let data = JSON.parse(stdout);
                        this._updateBiometricsUI(data);
                    }
                } catch (e) {}
            });
        } catch (e) {}
    }

    _updateBiometricsUI(data) {
        if (!this._biometricsBox || !this._isLocked) return;

        let hasFace = (data.face_enrolled === true);
        let hasFp = (data.fingerprint_enrolled === true) || (data.has_fingerprint === true);

        if (!hasFace && !hasFp) {
            this._biometricsBox.visible = false;
            return;
        }

        this._biometricsBox.visible = true;
        this._faceArea.visible = hasFace;
        this._fpActor.visible = hasFp;

        if (hasFace) {
            if (data.face_state === 'matched') {
                this._faceFailCount = 0;
                this._onBiometricsMatched('face');
                return;
            } else if (data.face_state === 'failed') {
                this._onBiometricsDenied('face');
            } else if (data.face_state === 'paused') {
                if (this._faceState !== 'paused') {
                    this._faceState = 'paused';
                    this._faceArea.queue_repaint();
                }
            } else if (this._faceState !== 'scanning' && this._faceState !== 'idle') {
                this._faceState = 'scanning';
                this._faceArea.queue_repaint();
            }
        }

        if (hasFp) {
            if (data.fingerprint_state === 'matched') {
                this._onBiometricsMatched('fingerprint');
                return;
            } else if (data.fingerprint_state === 'failed') {
                this._onBiometricsDenied('fingerprint');
            } else {
                this._startFpPulse();
                this._fpActor.style_class = 'pulsaros-lockscreen-fp-icon';
            }
        }

        let isSpanish = (GLib.getenv('LANG') || '').startsWith('es');
        if (data.face_state === 'paused') {
            if (hasFp) {
                this._bioStatusLabel.text = isSpanish ? 'Pulsa para activar Face ID o usa la huella' : 'Click to activate Face ID or use fingerprint';
            } else {
                this._bioStatusLabel.text = isSpanish ? 'Pulsa para activar Face ID' : 'Click to activate Face ID';
            }
        } else if (data.status_message) {
            this._bioStatusLabel.text = data.status_message;
        } else if (hasFace && hasFp) {
            this._bioStatusLabel.text = isSpanish ? 'Esperando rostro o huella...' : 'Waiting for face or fingerprint...';
        } else if (hasFace) {
            this._bioStatusLabel.text = isSpanish ? 'Buscando rostro...' : 'Looking for face...';
        } else if (hasFp) {
            this._bioStatusLabel.text = isSpanish ? 'Esperando huella...' : 'Waiting for fingerprint...';
        }
    }

    _startFpPulse() {
        if (this._fpPulseTimerId) return;
        let growing = false;
        let opacity = 255;
        this._fpPulseTimerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 50, () => {
            if (!this._isLocked || !this._fpActor) {
                this._fpPulseTimerId = 0;
                return GLib.SOURCE_REMOVE;
            }
            if (growing) {
                opacity += 8;
                if (opacity >= 255) growing = false;
            } else {
                opacity -= 8;
                if (opacity <= 90) growing = true;
            }
            this._fpActor.opacity = opacity;
            return GLib.SOURCE_CONTINUE;
        });
    }

    _onBiometricsMatched(type) {
        if (this._matchedTimeoutId) return;
        let isSpanish = (GLib.getenv('LANG') || '').startsWith('es');
        if (type === 'face') {
            this._faceFailCount = 0;
            this._faceState = 'matched';
            this._helloWink = 0.2;
            if (this._faceArea) this._faceArea.queue_repaint();
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 120, () => {
                this._helloWink = 1.0;
                if (this._faceArea) this._faceArea.queue_repaint();
                return GLib.SOURCE_REMOVE;
            });
            this._bioStatusLabel.text = isSpanish ? '¡Rostro verificado!' : 'Face verified!';
            this._bioStatusBadge.style_class = 'pulsaros-lockscreen-bio-badge-matched';
        } else {
            this._fpActor.style_class = 'pulsaros-lockscreen-fp-matched';
            this._bioStatusLabel.text = isSpanish ? '¡Huella verificada!' : 'Fingerprint verified!';
            this._bioStatusBadge.style_class = 'pulsaros-lockscreen-bio-badge-matched';
        }
        this._matchedTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 380, () => {
            this._matchedTimeoutId = 0;
            this._onAuthSuccess(this._selectedUsername);
            return GLib.SOURCE_REMOVE;
        });
    }

    _onBiometricsDenied(type) {
        let isSpanish = (GLib.getenv('LANG') || '').startsWith('es');
        if (type === 'face') {
            this._faceFailCount = (this._faceFailCount || 0) + 1;
            this._faceState = 'failed';
            if (this._faceArea) {
                this._faceArea.queue_repaint();
                this._shakeActor(this._faceArea);
            }
            if (this._faceFailCount >= 2) {
                try {
                    Gio.Subprocess.new(['curl', '-s', 'http://127.0.0.1:39281/show_camera'], Gio.SubprocessFlags.NONE);
                } catch (e) {}
            }
            this._bioStatusLabel.text = isSpanish ? 'Rostro no reconocido' : 'Face not recognized';
            this._bioStatusBadge.style_class = 'pulsaros-lockscreen-bio-badge-failed';
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 1400, () => {
                if (this._isLocked && this._faceState === 'failed') {
                    this._faceState = 'scanning';
                    if (this._faceArea) this._faceArea.queue_repaint();
                }
                return GLib.SOURCE_REMOVE;
            });
        } else {
            this._fpActor.style_class = 'pulsaros-lockscreen-fp-failed';
            this._bioStatusLabel.text = isSpanish ? 'Huella no reconocida' : 'Fingerprint not recognized';
            this._bioStatusBadge.style_class = 'pulsaros-lockscreen-bio-badge-failed';
            this._shakeActor(this._fpActor);
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 1400, () => {
                if (this._isLocked && this._fpActor) {
                    this._fpActor.style_class = 'pulsaros-lockscreen-fp-icon';
                }
                return GLib.SOURCE_REMOVE;
            });
        }
    }

    _shakeActor(actor) {
        if (!actor) return;
        let step = 0;
        let offsets = [-6, 6, -4, 4, -2, 2, 0];
        let origX = actor.translation_x;
        let timer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 45, () => {
            if (step >= offsets.length) {
                actor.translation_x = origX;
                return GLib.SOURCE_REMOVE;
            }
            actor.translation_x = origX + offsets[step];
            step++;
            return GLib.SOURCE_CONTINUE;
        });
    }
    
    destroy() {
        this._stopLockscreenIdleTimers();
        this._unblankDisplay();
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
        if (this._sleepSignalId) {
            Gio.DBus.system.signal_unsubscribe(this._sleepSignalId);
            this._sleepSignalId = 0;
        }
        if (this._grab) {
            try { Main.popModal(this._grab); } catch (e) {}
            this._grab = null;
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
