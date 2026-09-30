import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Gst from 'gi://Gst';
import GstApp from 'gi://GstApp';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

export class DesktopLiveWallpaperManager {
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
