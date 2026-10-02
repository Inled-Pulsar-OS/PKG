import St from 'gi://St';
import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Shell from 'gi://Shell';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';
import Pango from 'gi://Pango';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

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
        this._hasGrab = false;
        this._authenticating = false;
        this._timerId = 0;
        this._sizeChangedId = 0;
        this._sizeChangedId2 = 0;
        this._monitorContainers = [];
        this._clocks = [];
        this._passwordEntry = null;
        this._lockIdleTimerId = 0;
        this._lockIdleTimeoutSeconds = 60;
        this._stageEventId = 0;
        this._selectedUsername = GLib.get_user_name();
        this._userPickerBox = null;
        this._avatarWidget = null;
        this._nameLabel = null;

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

        // Click anywhere to focus password entry on primary monitor
        container.connect('button-press-event', () => {
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
    
    _resetLockIdleTimer() {
        if (!this._isLocked) return;
        if (this._lockIdleTimerId) {
            GLib.source_remove(this._lockIdleTimerId);
            this._lockIdleTimerId = 0;
        }
        this._lockIdleTimerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, this._lockIdleTimeoutSeconds, () => {
            this._lockIdleTimerId = 0;
            if (this._isLocked) {
                this._suspendOnLockIdle();
            }
            return GLib.SOURCE_REMOVE;
        });
    }

    _suspendOnLockIdle() {
        if (!this._isLocked) return;
        try {
            let bus = Gio.DBus.system;
            bus.call(
                'org.freedesktop.login1',
                '/org/freedesktop/login1',
                'org.freedesktop.login1.Manager',
                'Suspend',
                new GLib.Variant('(b)', [true]),
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                (connection, res) => {
                    try {
                        connection.call_finish(res);
                    } catch (e) {
                        GLib.spawn_command_line_async("systemctl suspend");
                    }
                }
            );
        } catch (e) {
            console.error("[LockScreen] Failed to trigger lockscreen idle suspend:", e);
            GLib.spawn_command_line_async("systemctl suspend");
        }
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
            // Already locked: ensure top of uiGroup stack and reset lock idle timer
            try {
                let parent = this.get_parent();
                if (parent) {
                    parent.set_child_at_index(this, -1);
                }
            } catch (e) {}
            this._resetLockIdleTimer();
            if (this._passwordEntry) {
                let activeText = this._passwordEntry.clutter_text || this._passwordEntry.clutterText || this._passwordEntry;
                if (activeText && activeText.grab_key_focus) activeText.grab_key_focus();
            }
            return;
        }
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
                if (activeText && activeText.grab_key_focus) activeText.grab_key_focus();
            }
            return GLib.SOURCE_REMOVE;
        });
        
        // Start live clock updates
        this._updateClock();
        this._timerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 1, () => {
            this._updateClock();
            return GLib.SOURCE_CONTINUE;
        });

        // Start lockscreen inactivity timer for automatic suspension
        this._resetLockIdleTimer();

        // Listen to global events to reset the inactivity timer
        if (!this._stageEventId) {
            this._stageEventId = global.stage.connect('captured-event', () => {
                if (this._isLocked) {
                    this._resetLockIdleTimer();
                }
                return Clutter.EVENT_PROPAGATE;
            });
        }
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

        // Clean lock idle timer and stage event listener
        if (this._lockIdleTimerId) {
            GLib.source_remove(this._lockIdleTimerId);
            this._lockIdleTimerId = 0;
        }
        if (this._stageEventId) {
            global.stage.disconnect(this._stageEventId);
            this._stageEventId = 0;
        }
    }
    
    _authenticate(password) {
        if (this._authenticating) return;
        this._authenticating = true;
        
        if (this._passwordEntry) {
            this._passwordEntry.set_reactive(false);
            this._passwordEntry.style_class = 'pulsaros-lockscreen-entry-authenticating';
        }
        
        let username = this._selectedUsername || GLib.get_user_name();
        
        // Check if the PAM service file is present. If not, fallback to developer passwords for local testing on host
        let pamFile = Gio.File.new_for_path('/etc/pam.d/pulsaros-lock');
        if (!pamFile.query_exists(null)) {
            console.warn("[LockScreen] PAM service '/etc/pam.d/pulsaros-lock' is missing. Falling back to developer passwords.");
            if (password === 'pulsar' || password === 'live' || password === 'jaime') {
                this._onAuthSuccess(username);
            } else {
                this._onAuthFailure();
            }
            return;
        }
        
        try {
            // Run pamtester asynchronously, piping the password via stdin
            let proc = Gio.Subprocess.new(
                ['/usr/bin/pamtester', 'pulsaros-lock', username, 'authenticate'],
                Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
            );
            
            proc.communicate_utf8_async(password + '\n', null, (obj, res) => {
                try {
                    let [ok, , stderrText] = obj.communicate_utf8_finish(res);
                    let success = ok && obj.get_successful();
                    if (!success) {
                        console.warn(`[LockScreen] pamtester auth failed for user '${username}': exit=${obj.get_exit_status()} stderr=${(stderrText || '').trim()}`);
                    }
                    if (success) {
                        this._onAuthSuccess(username);
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
            this._onAuthFailure();
        }
    }
    
    _onAuthSuccess(username) {
        this._authenticating = false;
        let sessionUser = GLib.get_user_name();
        if (username && username !== sessionUser) {
            this._switchUser();
        } else {
            this.unlock();
        }
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
        if (this._lockIdleTimerId) {
            GLib.source_remove(this._lockIdleTimerId);
            this._lockIdleTimerId = 0;
        }
        if (this._stageEventId) {
            global.stage.disconnect(this._stageEventId);
            this._stageEventId = 0;
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
