import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

export class ShelfHelper {
    constructor() {
        this.shelfDir = GLib.build_filenamev([GLib.get_user_data_dir(), 'notchnux', 'shelf']);
        this.notesFile = GLib.build_filenamev([GLib.get_user_data_dir(), 'notchnux', 'notes.txt']);
        
        this._ensureDirectories();
    }

    _ensureDirectories() {
        try {
            GLib.mkdir_with_parents(this.shelfDir, 0o755);
            
            // Ensure notes file exists
            let file = Gio.File.new_for_path(this.notesFile);
            if (!file.query_exists(null)) {
                file.replace_contents('', null, false, Gio.FileCreateFlags.NONE, null);
            }
        } catch (e) {
            console.error('NotchNux: Error creating storage directories', e);
        }
    }

    // --- Files Shelf API ---
    getFiles() {
        let list = [];
        try {
            let directory = Gio.File.new_for_path(this.shelfDir);
            let enumerator = directory.enumerate_children(
                'standard::name,standard::size,standard::content-type',
                Gio.FileQueryInfoFlags.NONE,
                null
            );

            let info;
            while ((info = enumerator.next_file(null)) !== null) {
                let name = info.get_name();
                let size = info.get_size();
                let contentType = info.get_content_type() || 'unknown';
                let filePath = GLib.build_filenamev([this.shelfDir, name]);
                let uri = `file://${filePath}`;

                // Resolve matching GIcon
                let iconName = 'text-x-generic-symbolic';
                let gicon = info.get_icon();
                if (gicon) {
                    let names = gicon.to_string().split(' ');
                    // Standard icon name fallback
                    iconName = names.find(n => n.endsWith('-symbolic')) || names[0] || 'text-x-generic-symbolic';
                }

                list.push({
                    name: name,
                    path: filePath,
                    uri: uri,
                    sizeStr: this._formatSize(size),
                    icon: iconName
                });
            }
        } catch (e) {
            console.error('NotchNux: Error enumerating shelf files', e);
        }
        return list;
    }

    addFile(srcPath) {
        try {
            // Support raw paths or file:// URIs
            if (srcPath.startsWith('file://')) {
                srcPath = srcPath.replace('file://', '');
            }
            
            // URI decoding for URL-encoded characters (like %20 for space)
            srcPath = GLib.uri_unescape_string(srcPath, null);

            let srcFile = Gio.File.new_for_path(srcPath);
            if (!srcFile.query_exists(null)) {
                return false;
            }

            let basename = srcFile.get_basename();
            let destPath = GLib.build_filenamev([this.shelfDir, basename]);
            let destFile = Gio.File.new_for_path(destPath);

            srcFile.copy(destFile, Gio.FileCopyFlags.OVERWRITE, null, null);
            return true;
        } catch (e) {
            console.error(`NotchNux: Failed to add file ${srcPath} to shelf`, e);
        }
        return false;
    }

    addTextSnippet(text) {
        try {
            if (!text || typeof text !== 'string' || text.trim().length === 0) return false;
            let clean = text.trim();
            let preview = clean.slice(0, 24).replace(/[\r\n\t\/\\:*?"<>|]/g, ' ').trim();
            if (!preview) preview = 'Snippet';
            let filename = `${preview}.txt`;
            let destPath = GLib.build_filenamev([this.shelfDir, filename]);
            let destFile = Gio.File.new_for_path(destPath);
            destFile.replace_contents(clean, null, false, Gio.FileCreateFlags.NONE, null);
            return true;
        } catch (e) {
            console.error('NotchNux: Failed to add text snippet to shelf', e);
            return false;
        }
    }

    deleteFile(filePath) {
        try {
            let file = Gio.File.new_for_path(filePath);
            if (file.query_exists(null)) {
                file.delete(null);
                return true;
            }
        } catch (e) {
            console.error(`NotchNux: Failed to delete shelf file ${filePath}`, e);
        }
        return false;
    }

    clearShelf() {
        try {
            let directory = Gio.File.new_for_path(this.shelfDir);
            let enumerator = directory.enumerate_children(
                'standard::name',
                Gio.FileQueryInfoFlags.NONE,
                null
            );

            let info;
            while ((info = enumerator.next_file(null)) !== null) {
                let name = info.get_name();
                let filePath = GLib.build_filenamev([this.shelfDir, name]);
                Gio.File.new_for_path(filePath).delete(null);
            }
        } catch (e) {
            console.error('NotchNux: Failed to clear shelf', e);
        }
    }

    openFile(filePath) {
        try {
            Gio.AppInfo.launch_default_for_uri(`file://${filePath}`, null);
        } catch (e) {
            console.error(`NotchNux: Failed to open file ${filePath}`, e);
        }
    }

    showInFiles(filePath) {
        try {
            // E.g. spawn nautilus select Command
            GLib.spawn_command_line_async(`nautilus --select "${filePath}"`);
        } catch (e) {
            console.error(`NotchNux: Failed to open Nautilus selection for ${filePath}`, e);
        }
    }

    copyToClipboard(text) {
        try {
            let clipboard = St.Clipboard.get_default();
            clipboard.set_text(St.ClipboardType.CLIPBOARD, text);
        } catch (e) {
            console.error('NotchNux: Failed to set clipboard text', e);
        }
    }

    // Put the actual file on the Wayland clipboard as a copied *file* (not just
    // its path) so it can be pasted into a file manager or attached in chat
    // apps. St.Clipboard can only carry text, so we shell out to wl-copy with
    // the file's real MIME type and its bytes on stdin.
    //
    // Returns true if wl-copy was found and spawned, false otherwise (the
    // caller falls back to copying the URI as text). wl-copy must stay resident
    // to keep serving the selection, so it is left running (it exits when the
    // selection is replaced) — we don't wait on it.
    copyFileToClipboard(filePath) {
        try {
            let file = Gio.File.new_for_path(filePath);
            if (!file.query_exists(null))
                return false;
            // wl-copy needs a concrete mime type to advertise; text/uri-list
            // is what file managers read for a "copied file" paste.
            let uri = file.get_uri();
            let [ok] = GLib.spawn_async(
                null,
                ['wl-copy', '--type', 'text/uri-list', uri + '\r\n'],
                null,
                GLib.SpawnFlags.SEARCH_PATH,
                null
            );
            return ok;
        } catch (e) {
            console.error(`NotchNux: Failed to copy file to clipboard ${filePath}`, e);
            return false;
        }
    }

    // Pull whatever file(s) are on the Wayland clipboard (e.g. a Ctrl+C'd file
    // in Nautilus, which is advertised as text/uri-list) and add them to the
    // shelf. This is our stand-in for external drag-and-drop, which Wayland
    // won't route into a shell extension's own actors. Async because we shell
    // out to wl-paste and read its output. `callback(addedCount)` runs on the
    // main loop; addedCount is -1 if wl-paste isn't available.
    pasteFilesFromClipboard(callback) {
        this.pasteFromClipboardOrDnd(callback);
    }

    // Ask the desktop portal where to put a copy of a shelf item. This is the
    // working replacement for dragging a row out of the shelf: a shell drag
    // never leaves the shell, since Mutter is given no wl_data_source for it,
    // so the only ways to hand a file elsewhere are to write a copy to a
    // location the user picks, or to put the file on the clipboard.
    // `callback(addedCount)` runs on the main loop: 1 on a successful copy,
    // 0 if the dialog was cancelled or the copy failed.
    saveCopyToChosenPath(filePath, callback) {
        let src = Gio.File.new_for_path(filePath);
        if (!src.query_exists(null)) {
            if (callback) callback(0);
            return;
        }

        let bus = Gio.DBus.session;
        // Same portal handshake as pickFilesIntoShelf: subscribe to the Request
        // object before the call, since the token makes its path predictable.
        let token = 'notchnux_' + Math.floor(Math.random() * 0x7fffffff);
        let sender = bus.get_unique_name().replace(/^:/, '').replace(/\./g, '_');
        let requestPath = `/org/freedesktop/portal/desktop/request/${sender}/${token}`;

        let subId = 0;
        let finish = (n) => {
            if (subId) { bus.signal_unsubscribe(subId); subId = 0; }
            if (callback) callback(n);
        };

        subId = bus.signal_subscribe(
            'org.freedesktop.portal.Desktop',
            'org.freedesktop.portal.Request',
            'Response',
            requestPath,
            null,
            Gio.DBusSignalFlags.NONE,
            (conn, sender_, path, iface, signal, params) => {
                let [responseCode, results] = params.deepUnpack();
                if (responseCode !== 0) { finish(0); return; }
                let uris = results['uris'] ? results['uris'].deepUnpack() : [];
                if (!uris.length) { finish(0); return; }
                let dest = Gio.File.new_for_uri(uris[0]);
                try {
                    src.copy(dest, Gio.FileCopyFlags.OVERWRITE, null, null);
                    finish(1);
                } catch (e) {
                    console.error('NotchNux: could not save copy', e);
                    finish(0);
                }
            }
        );

        // GJS refuses to pack an already-built GLib.Variant into another one
        // ("not a subclass of GObject_Struct"), so the option dict is unpacked
        // before it goes into the call tuple.
        let options = new GLib.Variant('a{sv}', {
            handle_token: new GLib.Variant('s', token),
            current_name: new GLib.Variant('s', src.get_basename()),
            current_folder: new GLib.Variant('ay', src.get_parent().get_uri()),
            modal: new GLib.Variant('b', true),
        }).deepUnpack();

        bus.call(
            'org.freedesktop.portal.Desktop',
            '/org/freedesktop/portal/desktop',
            'org.freedesktop.portal.FileChooser',
            'SaveFile',
            new GLib.Variant('(ssa{sv})', ['', 'Save a copy', options]),
            new GLib.VariantType('(o)'),
            Gio.DBusCallFlags.NONE,
            -1,
            null,
            (src_, res) => {
                try {
                    bus.call_finish(res);
                } catch (e) {
                    console.error('NotchNux: FileChooser SaveFile call failed', e);
                    finish(0);
                }
            }
        );
    }

    // Pull whatever file(s), raw image bytes, or text are on the Wayland
    // clipboard or primary selection and add them to the shelf.
    // Calls `callback(addedCount, type)` where type is 'file' | 'image' | 'text'.
    pasteFromClipboardOrDnd(callback) {
        let done = (n, t = null) => { if (callback) callback(n, t); };
        let launcher = new Gio.SubprocessLauncher({
            flags: Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_SILENCE,
        });

        // 1. Check URI lists on primary (DND offer) and clipboard
        let tryUriList = (isPrimary, next) => {
            let args = isPrimary ? ['wl-paste', '--primary', '--no-newline', '--type', 'text/uri-list']
                                 : ['wl-paste', '--no-newline', '--type', 'text/uri-list'];
            try {
                let proc = launcher.spawnv(args);
                proc.communicate_utf8_async(null, null, (p, res) => {
                    let added = 0;
                    try {
                        let [, stdout] = p.communicate_utf8_finish(res);
                        let uris = (stdout || '').split(/\r?\n/)
                            .map(s => s.trim())
                            .filter(s => s.length > 0 && !s.startsWith('#'));
                        for (let uri of uris) {
                            if (this.addFile(uri)) added++;
                        }
                    } catch (_) {}
                    if (added > 0) done(added, 'file');
                    else next();
                });
            } catch (_) { next(); }
        };

        // 2. Check for copied raw image bytes (e.g. screenshot, web image copy)
        let tryImage = (next) => {
            let filename = `Image_${new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)}.png`;
            let destPath = GLib.build_filenamev([this.shelfDir, filename]);
            try {
                let proc = launcher.spawnv(['wl-paste', '--no-newline', '--type', 'image/png']);
                proc.communicate_async(null, null, (p, res) => {
                    try {
                        let [, stdoutBytes] = p.communicate_finish(res);
                        if (stdoutBytes && stdoutBytes.get_size() > 64) {
                            let destFile = Gio.File.new_for_path(destPath);
                            destFile.replace_contents_bytes_async(stdoutBytes, null, false, Gio.FileCreateFlags.NONE, null, (f, r) => {
                                try {
                                    f.replace_contents_finish(r);
                                    done(1, 'image');
                                } catch (_) {
                                    next();
                                }
                            });
                            return;
                        }
                    } catch (_) {}
                    next();
                });
            } catch (_) { next(); }
        };

        // 3. Check for text/plain or x-special/gnome-copied-files
        let tryText = () => {
            try {
                let proc = launcher.spawnv(['wl-paste', '--no-newline']);
                proc.communicate_utf8_async(null, null, (p, res) => {
                    try {
                        let [, stdout] = p.communicate_utf8_finish(res);
                        let text = (stdout || '').trim();
                        if (text.length > 0) {
                            let added = 0;
                            let lines = text.split(/\r?\n/);
                            for (let line of lines) {
                                let l = line.trim();
                                if (l.startsWith('file://')) {
                                    if (this.addFile(l)) added++;
                                }
                            }
                            if (added > 0) {
                                done(added, 'file');
                                return;
                            }
                            if (this.addTextSnippet(text)) {
                                done(1, 'text');
                                return;
                            }
                        }
                    } catch (_) {}
                    done(0, null);
                });
            } catch (_) { done(0, null); }
        };

        tryUriList(true, () => {
            tryUriList(false, () => {
                tryImage(() => {
                    tryText();
                });
            });
        });
    }

    // Open the desktop portal's file chooser and add every selected file to
    // the shelf. Async: `callback(addedCount)` runs on the main loop once the
    // dialog closes. Uses org.freedesktop.portal.FileChooser directly so we
    // don't depend on an external picker binary.
    pickFilesIntoShelf(callback) {
        let bus = Gio.DBus.session;
        // The portal replies asynchronously on a Request object whose path it
        // returns; we must be subscribed to that object's Response signal
        // before (or right as) the call completes. The token makes the path
        // predictable per the portal spec, so we can subscribe up front.
        let token = 'notchnux_' + Math.floor(Math.random() * 0x7fffffff);
        let sender = bus.get_unique_name().replace(/^:/, '').replace(/\./g, '_');
        let requestPath = `/org/freedesktop/portal/desktop/request/${sender}/${token}`;

        let subId = 0;
        let finish = (added) => {
            if (subId) { bus.signal_unsubscribe(subId); subId = 0; }
            if (callback) callback(added);
        };

        subId = bus.signal_subscribe(
            'org.freedesktop.portal.Desktop',
            'org.freedesktop.portal.Request',
            'Response',
            requestPath,
            null,
            Gio.DBusSignalFlags.NONE,
            (conn, sender_, path, iface, signal, params) => {
                let [responseCode, results] = params.deepUnpack();
                // responseCode: 0 = ok, 1 = cancelled, 2 = other error.
                if (responseCode !== 0) { finish(0); return; }
                let urisVariant = results['uris'];
                let uris = urisVariant ? urisVariant.deepUnpack() : [];
                let added = 0;
                for (let uri of uris)
                    if (this.addFile(uri)) added++;
                finish(added);
            }
        );

        // Unpack before packing: GJS cannot nest one GLib.Variant inside another, it
        // throws "not a subclass of GObject_Struct, it's a GIRepositoryFunction".
        let options = new GLib.Variant('a{sv}', {
            handle_token: new GLib.Variant('s', token),
            multiple: new GLib.Variant('b', true),
        }).deepUnpack();

        bus.call(
            'org.freedesktop.portal.Desktop',
            '/org/freedesktop/portal/desktop',
            'org.freedesktop.portal.FileChooser',
            'OpenFile',
            new GLib.Variant('(ssa{sv})', ['', 'Add to shelf', options]),
            new GLib.VariantType('(o)'),
            Gio.DBusCallFlags.NONE,
            -1,
            null,
            (src, res) => {
                try {
                    bus.call_finish(res);
                } catch (e) {
                    console.error('NotchNux: FileChooser portal call failed', e);
                    finish(0);
                }
            }
        );
    }

    // --- Sticky Notes API ---
    loadNotes() {
        try {
            let file = Gio.File.new_for_path(this.notesFile);
            let [success, contents] = file.load_contents(null);
            if (success) {
                return new TextDecoder('utf-8').decode(contents);
            }
        } catch (e) {
            console.error('NotchNux: Error loading notes file', e);
        }
        return '';
    }

    saveNotes(text) {
        try {
            let file = Gio.File.new_for_path(this.notesFile);
            file.replace_contents(
                text,
                null,
                false,
                Gio.FileCreateFlags.NONE,
                null
            );
        } catch (e) {
            console.error('NotchNux: Error saving notes file', e);
        }
    }

    // --- FlyDrop Integration (LocalSend protocol via es.pulsaros.FlyDrop) ---
    static FLYDROP_NAME = 'es.pulsaros.FlyDrop';
    static FLYDROP_PATH = '/es/pulsaros/FlyDrop';
    static FLYDROP_IFACE = 'es.pulsaros.FlyDrop';

    static GSC_NAME = 'org.gnome.Shell.Extensions.GSConnect';
    static GSC_BASE = '/org/gnome/Shell/Extensions/GSConnect';

    // Return the currently reachable devices as [{ id, name, type, model, ip, port, isFlyDrop }].
    getShareDevices() {
        let out = [];
        let bus = Gio.DBus.session;

        // FlyDrop / LocalSend devices
        try {
            if (this.isFlyDropAvailable()) {
                let reply = bus.call_sync(
                    ShelfHelper.FLYDROP_NAME,
                    ShelfHelper.FLYDROP_PATH,
                    ShelfHelper.FLYDROP_IFACE,
                    'GetDiscoveredDevices',
                    null,
                    new GLib.VariantType('(s)'),
                    Gio.DBusCallFlags.NONE,
                    1500,
                    null
                );
                let [jsonStr] = reply.deepUnpack();
                let devs = JSON.parse(jsonStr || '[]');
                for (let d of devs) {
                    let type = d.deviceType || 'desktop';
                    if (type === 'mobile') type = 'phone';
                    out.push({
                        id: d.ip,
                        ip: d.ip,
                        port: d.port || 53317,
                        name: d.alias || d.ip,
                        model: d.deviceModel || '',
                        type: type,
                        isFlyDrop: true,
                    });
                }
            }
        } catch (e) {
            console.error('NotchNux: Error getting FlyDrop devices', e);
        }

        return out;
    }

    isFlyDropAvailable() {
        try {
            let bus = Gio.DBus.session;
            let reply = bus.call_sync(
                'org.freedesktop.DBus',
                '/org/freedesktop/DBus',
                'org.freedesktop.DBus',
                'NameHasOwner',
                new GLib.Variant('(s)', [ShelfHelper.FLYDROP_NAME]),
                new GLib.VariantType('(b)'),
                Gio.DBusCallFlags.NONE,
                -1,
                null
            );
            return reply.deepUnpack()[0];
        } catch (e) {
            return false;
        }
    }

    isGSConnectAvailable() {
        try {
            let bus = Gio.DBus.session;
            let reply = bus.call_sync(
                'org.freedesktop.DBus',
                '/org/freedesktop/DBus',
                'org.freedesktop.DBus',
                'NameHasOwner',
                new GLib.Variant('(s)', [ShelfHelper.GSC_NAME]),
                new GLib.VariantType('(b)'),
                Gio.DBusCallFlags.NONE,
                -1,
                null
            );
            return reply.deepUnpack()[0];
        } catch (e) {
            return false;
        }
    }

    isShareServiceAvailable() {
        return this.isFlyDropAvailable() || this.isGSConnectAvailable();
    }

    triggerScan() {
        try {
            Gio.DBus.session.call(
                ShelfHelper.FLYDROP_NAME,
                ShelfHelper.FLYDROP_PATH,
                ShelfHelper.FLYDROP_IFACE,
                'TriggerScan',
                null,
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                null
            );
        } catch (e) {
            console.error('NotchNux: FlyDrop TriggerScan error', e);
        }
    }

    openDownloadsFolder() {
        try {
            if (this.isFlyDropAvailable()) {
                Gio.DBus.session.call(
                    ShelfHelper.FLYDROP_NAME,
                    ShelfHelper.FLYDROP_PATH,
                    ShelfHelper.FLYDROP_IFACE,
                    'OpenDownloadsFolder',
                    null,
                    null,
                    Gio.DBusCallFlags.NONE,
                    -1,
                    null,
                    null
                );
            } else {
                GLib.spawn_command_line_async(`xdg-open "${GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOWNLOAD)}"`);
            }
        } catch (e) {
            GLib.spawn_command_line_async(`xdg-open "${GLib.get_home_dir()}/Downloads"`);
        }
    }

    openFlyDropSettings() {
        try {
            Gio.DBus.session.call(
                ShelfHelper.FLYDROP_NAME,
                ShelfHelper.FLYDROP_PATH,
                ShelfHelper.FLYDROP_IFACE,
                'OpenSettingsDialog',
                null,
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                null
            );
        } catch (e) {
            console.error('NotchNux: OpenSettingsDialog error', e);
        }
    }

    openSendDialog(filePaths) {
        try {
            let paths = Array.isArray(filePaths) ? filePaths : [filePaths];
            Gio.DBus.session.call(
                ShelfHelper.FLYDROP_NAME,
                ShelfHelper.FLYDROP_PATH,
                ShelfHelper.FLYDROP_IFACE,
                'OpenSendDialog',
                new GLib.Variant('(s)', [JSON.stringify(paths)]),
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                (src, res) => {
                    try { src.call_finish(res); } catch (_) {}
                }
            );
            return true;
        } catch (e) {
            console.error('NotchNux: OpenSendDialog error', e);
            return false;
        }
    }

    // Send a shelf file to the given device (FlyDrop or GSConnect)
    sendFileToDevice(devOrIp, filePath) {
        let paths = Array.isArray(filePath) ? filePath : [filePath];
        let targetIp = (typeof devOrIp === 'string') ? devOrIp : (devOrIp.ip || devOrIp.id || devOrIp.path);
        let isFlyDrop = (typeof devOrIp === 'object' && devOrIp.isFlyDrop !== undefined) ? devOrIp.isFlyDrop : (this.isFlyDropAvailable() && !targetIp.startsWith('/'));

        if (isFlyDrop) {
            try {
                Gio.DBus.session.call(
                    ShelfHelper.FLYDROP_NAME,
                    ShelfHelper.FLYDROP_PATH,
                    ShelfHelper.FLYDROP_IFACE,
                    'SendFiles',
                    new GLib.Variant('(ss)', [targetIp, JSON.stringify(paths)]),
                    null,
                    Gio.DBusCallFlags.NONE,
                    -1,
                    null,
                    (src, res) => {
                        try { src.call_finish(res); } catch (e) { console.error('NotchNux: FlyDrop SendFiles error', e); }
                    }
                );
                return true;
            } catch (e) {
                console.error(`NotchNux: Failed to send via FlyDrop to ${targetIp}`, e);
                return false;
            }
        } else {
            // GSConnect fallback
            try {
                let uri = Gio.File.new_for_path(paths[0]).get_uri();
                let param = new GLib.Variant('(sb)', [uri, false]);
                Gio.DBus.session.call(
                    ShelfHelper.GSC_NAME,
                    targetIp,
                    'org.gtk.Actions',
                    'Activate',
                    new GLib.Variant('(sava{sv})', ['shareFile', [param], {}]),
                    null,
                    Gio.DBusCallFlags.NONE,
                    -1,
                    null,
                    (src, res) => {
                        try { src.call_finish(res); } catch (e) { console.error('NotchNux: GSConnect shareFile failed', e); }
                    }
                );
                return true;
            } catch (e) {
                console.error(`NotchNux: Failed to send via GSConnect to ${targetIp}`, e);
                return false;
            }
        }
    }

    // Send plain text snippet directly to a device via FlyDrop
    sendTextToDevice(devOrIp, text) {
        let targetIp = (typeof devOrIp === 'string') ? devOrIp : (devOrIp.ip || devOrIp.id || devOrIp.path);
        let isFlyDrop = (typeof devOrIp === 'object' && devOrIp.isFlyDrop !== undefined) ? devOrIp.isFlyDrop : (this.isFlyDropAvailable() && !targetIp.startsWith('/'));

        if (isFlyDrop) {
            try {
                Gio.DBus.session.call(
                    ShelfHelper.FLYDROP_NAME,
                    ShelfHelper.FLYDROP_PATH,
                    ShelfHelper.FLYDROP_IFACE,
                    'SendText',
                    new GLib.Variant('(ss)', [targetIp, text]),
                    null,
                    Gio.DBusCallFlags.NONE,
                    -1,
                    null,
                    (src, res) => {
                        try { src.call_finish(res); } catch (e) { console.error('NotchNux: FlyDrop SendText error', e); }
                    }
                );
                return true;
            } catch (e) {
                console.error(`NotchNux: Failed to send text via FlyDrop to ${targetIp}`, e);
                return false;
            }
        }
        return false;
    }

    // Subscribe to FlyDrop transfer signals
    subscribeTransferSignals(callbacks = {}) {
        let bus = Gio.DBus.session;
        let subIds = [];

        if (callbacks.onProgress) {
            let id = bus.signal_subscribe(
                ShelfHelper.FLYDROP_NAME,
                ShelfHelper.FLYDROP_IFACE,
                'TransferProgress',
                ShelfHelper.FLYDROP_PATH,
                null,
                Gio.DBusSignalFlags.NONE,
                (conn, sender, path, iface, signal, params) => {
                    try {
                        let [sessionId, progress, status, currentFile, speedStr] = params.deepUnpack();
                        callbacks.onProgress({ sessionId, progress, status, currentFile, speedStr });
                    } catch (e) {
                        console.error('NotchNux: TransferProgress signal error', e);
                    }
                }
            );
            subIds.push(id);
        }

        if (callbacks.onCompleted) {
            let id = bus.signal_subscribe(
                ShelfHelper.FLYDROP_NAME,
                ShelfHelper.FLYDROP_IFACE,
                'TransferCompleted',
                ShelfHelper.FLYDROP_PATH,
                null,
                Gio.DBusSignalFlags.NONE,
                (conn, sender, path, iface, signal, params) => {
                    try {
                        let [sessionId, success, message] = params.deepUnpack();
                        callbacks.onCompleted({ sessionId, success, message });
                    } catch (e) {
                        console.error('NotchNux: TransferCompleted signal error', e);
                    }
                }
            );
            subIds.push(id);
        }

        if (callbacks.onDevicesChanged) {
            for (let sig of ['DevicesChanged', 'DeviceFound', 'DeviceLost']) {
                let id = bus.signal_subscribe(
                    ShelfHelper.FLYDROP_NAME,
                    ShelfHelper.FLYDROP_IFACE,
                    sig,
                    ShelfHelper.FLYDROP_PATH,
                    null,
                    Gio.DBusSignalFlags.NONE,
                    () => {
                        try { callbacks.onDevicesChanged(); } catch (_) {}
                    }
                );
                subIds.push(id);
            }
        }

        return () => {
            for (let id of subIds) {
                try { bus.signal_unsubscribe(id); } catch (_) {}
            }
        };
    }

    // --- Formatting Helper ---
    _formatSize(bytes) {
        if (!bytes || bytes <= 0) return '0 B';
        let k = 1024;
        let sizes = ['B', 'KB', 'MB', 'GB'];
        let i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }
}
