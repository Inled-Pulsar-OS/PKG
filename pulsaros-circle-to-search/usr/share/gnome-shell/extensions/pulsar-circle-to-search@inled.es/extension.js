import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';
import Shell from 'gi://Shell';
import Clutter from 'gi://Clutter';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import { getPointerWatcher } from 'resource:///org/gnome/shell/ui/pointerWatcher.js';

import { ScreenshotOCRController } from './ocr.js';
import { LensUploader } from './uploader.js';

const Tooltip = GObject.registerClass(
class Tooltip extends St.Label {
    _init(widget, text) {
        super._init({
            text,
            style_class: 'screenshot-ui-tooltip',
            visible: false,
        });

        this._widget = widget;

        // Auto-disconnect tooltip signals with connectObject.
        widget.connectObject('notify::hover', () => {
            if (widget.hover)
                this._show(widget);
            else
                this._hide();
        }, this);

        // Destroy tooltip when its owner disappears.
        widget.connectObject('destroy', () => this.destroy(), this);
    }

    _show(widget) {
        // Use transition delay, not timeout source.
        const extents = widget.get_transformed_extents();
        const x = Math.floor(extents.get_x() + (extents.get_width() - this.width) / 2);
        const y = extents.get_y() + extents.get_height() + 6;

        this.remove_all_transitions();
        this.set_position(x, y);
        this.opacity = 0;
        this.show();
        this.ease({
            opacity: 255,
            delay: 500,
            duration: 120,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
        });
    }

    _hide() {
        this.remove_all_transitions();
        this.hide();
    }

    vfunc_destroy() {
        this._hide();
        this._widget = null;

        super.vfunc_destroy();
    }
});

export default class ShotzyExtension extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._uploader = new LensUploader();
        this._ocrController = new ScreenshotOCRController(
            this._settings,
            text => this._showScreenshotMessage(text)
        );
        this._lensButtonClickedId = 0;
        this._sayriButtonClickedId = 0;
        this._qrButtonClickedId = 0;
        this._settingsChangedId = 0;
        this._uiOpenOriginal = null;
        this._areaSelectorUpdateOriginal = null;
        this._uiClosedId = 0;
        this._areaSelectorDragEndedId = 0;
        this._tooltips = [];
        this._overlayMessage = null;
        this._overlayMessageTimeoutId = 0;

        this._setupShakeDetector();
        this._hookScreenshotUI();
        this._injectLensButton();
        this._settingsChangedId = this._settings.connect('changed', (_settings, key) => {
            if (key === 'show-google-lens-button' || key === 'show-qr-button')
                this._syncActionButtons();
        });
    }

    _setupShakeDetector() {
        try {
            this._pointerWatcher = getPointerWatcher();
            const startTime = Date.now();
            let lastX = null;
            let lastY = null;
            let lastMoveTime = 0;
            let strokeStartPos = 0;
            let strokeStartTime = 0;
            let strokeDir = 0;
            let reversals = [];
            let lastTrigger = 0;

            this._shakeWatch = this._pointerWatcher.addWatch(16, (x, y) => {
                const now = Date.now();
                // 1. Startup Warmup: Ignore cursor coordinates during first 2.0s
                if (now - startTime < 2000) {
                    lastX = x;
                    lastY = y;
                    return;
                }

                if (lastX === null || lastY === null) {
                    lastX = x;
                    lastY = y;
                    lastMoveTime = now;
                    strokeStartPos = x;
                    strokeStartTime = now;
                    return;
                }

                const dt = now - lastMoveTime;
                lastMoveTime = now;

                // 2. Idle timeout: If mouse stopped or stuttered (> 100ms between ticks), reset shake
                if (dt > 100) {
                    strokeDir = 0;
                    reversals = [];
                    lastX = x;
                    lastY = y;
                    strokeStartPos = x;
                    strokeStartTime = now;
                    return;
                }

                const dx = x - lastX;
                lastX = x;
                lastY = y;

                if (Math.abs(dx) < 6) return;
                const d = dx > 0 ? 1 : -1;

                if (strokeDir === 0) {
                    strokeDir = d;
                    strokeStartPos = x;
                    strokeStartTime = now;
                    return;
                }

                // If direction reversed
                if (d !== strokeDir) {
                    const strokeDist = Math.abs(x - strokeStartPos);
                    const strokeDuration = Math.max(now - strokeStartTime, 1);
                    const speed = (strokeDist / strokeDuration) * 1000; // px/sec

                    // Must be a rapid, intentional twitch stroke:
                    // - Stroke length >= 80px
                    // - Stroke duration <= 130ms (fast back-and-forth)
                    // - Speed >= 1200 px/sec
                    if (strokeDist >= 80 && strokeDuration <= 130 && speed >= 1200) {
                        reversals.push({ time: now, dist: strokeDist });
                        reversals = reversals.filter(r => now - r.time <= 450);
                        const totalDist = reversals.reduce((acc, r) => acc + r.dist, 0);

                        // Require at least 6 fast reversals (3 full vigorous shakes) and >= 480px total distance
                        if (reversals.length >= 6 && totalDist >= 480) {
                            if (now - lastTrigger >= 3500) {
                                lastTrigger = now;
                                reversals = [];
                                strokeDir = 0;
                                if (Main.screenshotUI && !Main.screenshotUI._isOpen) {
                                    Main.screenshotUI.open();
                                }
                            }
                        }
                    } else {
                        // Slow or long stroke is normal mouse movement -> clear prior reversals
                        reversals = [];
                    }

                    strokeDir = d;
                    strokeStartPos = x;
                    strokeStartTime = now;
                }
            });
        } catch (e) {
            log(`Pulsar Circle to Search shake detector error: ${e.message}`);
        }
    }

    disable() {
        if (this._shakeWatch && this._pointerWatcher) {
            this._pointerWatcher._removeWatch(this._shakeWatch);
            this._shakeWatch = null;
        }

        const ui = Main.screenshotUI;
        
        if (ui) {
            if (this._uiClosedId) {
                ui.disconnect(this._uiClosedId);
                this._uiClosedId = 0;
            }
            if (ui._areaSelector && this._areaSelectorDragEndedId) {
                ui._areaSelector.disconnect(this._areaSelectorDragEndedId);
                this._areaSelectorDragEndedId = 0;
            }
            this._unhookScreenshotUI(ui);
        }

        if (this._lensWrapper) {
            if (ui && ui._panel) {
                ui._panel.remove_child(this._lensWrapper);
                
                if (ui._typeButtonContainer && ui._bottomRowContainer) {
                    if (this._lensInnerVBox) {
                        this._lensInnerVBox.remove_child(ui._typeButtonContainer);
                        this._lensInnerVBox.remove_child(ui._bottomRowContainer);
                    }
                    ui._panel.add_child(ui._typeButtonContainer);
                    ui._panel.add_child(ui._bottomRowContainer);
                }
            }
            this._lensInnerVBox?.destroy();
            this._lensInnerVBox = null;
            this._lensSideBox?.destroy();
            this._lensSideBox = null;
            this._lensWrapper.destroy();
            this._lensWrapper = null;
        }

        if (this._sayriButton) {
            if (this._sayriButtonClickedId) {
                this._sayriButton.disconnect(this._sayriButtonClickedId);
                this._sayriButtonClickedId = 0;
            }
            this._sayriButton.destroy();
            this._sayriButton = null;
        }

        if (this._lensButton) {
            if (this._lensButtonClickedId) {
                this._lensButton.disconnect(this._lensButtonClickedId);
                this._lensButtonClickedId = 0;
            }
            this._lensButton.destroy();
            this._lensButton = null;
        }

        if (this._copyTextButton) {
            if (this._copyTextButtonClickedId) {
                this._copyTextButton.disconnect(this._copyTextButtonClickedId);
                this._copyTextButtonClickedId = 0;
            }
            this._copyTextButton.destroy();
            this._copyTextButton = null;
        }

        if (this._qrButton) {
            if (this._qrButtonClickedId) {
                this._qrButton.disconnect(this._qrButtonClickedId);
                this._qrButtonClickedId = 0;
            }
            this._qrButton.destroy();
            this._qrButton = null;
        }

        if (this._tooltips) {
            this._tooltips.forEach(t => t.destroy());
            this._tooltips = [];
        }

        this._destroyOverlayMessage();

        if (this._settings && this._settingsChangedId) {
            this._settings.disconnect(this._settingsChangedId);
            this._settingsChangedId = 0;
        }

        this._ocrController?.destroy();
        this._ocrController = null;
        this._settings = null;
        this._uploader = null;
    }

    _hookScreenshotUI() {
        const ui = Main.screenshotUI;
        if (!ui)
            return;

        this._ocrController.ensureAttached(ui);

        if (!this._uiOpenOriginal) {
            this._uiOpenOriginal = ui.open;
            ui.open = async (...args) => {
                const result = await this._uiOpenOriginal.apply(ui, args);

                this._ocrController.ensureAttached(ui);
                if (ui._selectionButton?.checked) {
                    this._ocrController.start(ui).catch(e => {
                        log(`Shotzy: OCR start failed: ${e.message}`);
                    });
                } else {
                    this._ocrController.reset();
                }

                return result;
            };
        }

        if (ui._areaSelector && !this._areaSelectorUpdateOriginal) {
            this._areaSelectorUpdateOriginal = ui._areaSelector._updateSelectionRect;
            ui._areaSelector._updateSelectionRect = (...args) => {
                const result = this._areaSelectorUpdateOriginal.apply(ui._areaSelector, args);
                this._ocrController.refreshSelection(ui);
                return result;
            };
        }

        if (!this._uiClosedId) {
            this._uiClosedId = ui.connect('closed', () => {
                this._ocrController.reset();
            });
        }

        if (ui._areaSelector && !this._areaSelectorDragEndedId) {
            this._areaSelectorDragEndedId = ui._areaSelector.connect('drag-ended', () => {
                this._ocrController.refineSelection(ui).catch(e => {
                    log(`Shotzy: OCR selection refine failed: ${e.message}`);
                });
            });
        }
    }

    _unhookScreenshotUI(ui) {
        if (this._uiOpenOriginal) {
            ui.open = this._uiOpenOriginal;
            this._uiOpenOriginal = null;
        }
        if (ui._areaSelector && this._areaSelectorUpdateOriginal) {
            ui._areaSelector._updateSelectionRect = this._areaSelectorUpdateOriginal;
            this._areaSelectorUpdateOriginal = null;
        }
    }

    _injectLensButton() {
        const ui = Main.screenshotUI;
        if (!ui || !ui._panel) return;

        if (!this._actionsRow) {
            this._actionsRow = new St.BoxLayout({
                style_class: 'screenshot-ui-actions-row',
                vertical: false,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER,
                style: `
                    margin-top: 14px;
                    spacing: 16px;
                    padding: 6px 14px;
                    background-color: rgba(255, 255, 255, 0.05);
                    border: 1px solid rgba(255, 255, 255, 0.08);
                    border-radius: 9999px;
                `,
            });
        }

        // 1. Sayri Button (Clean PNG, large size)
        if (!this._sayriButton) {
            let iconChild;
            const sayriPng = '/usr/share/icons/hicolor/48x48/apps/sayri-tray.png';
            if (GLib.file_test(sayriPng, GLib.FileTest.EXISTS)) {
                iconChild = new St.Icon({
                    gicon: Gio.FileIcon.new(Gio.File.new_for_path(sayriPng)),
                    icon_size: 28,
                });
            } else {
                iconChild = new St.Icon({
                    icon_name: 'starred-symbolic',
                    icon_size: 28,
                });
            }

            this._sayriButton = new St.Button({
                style_class: 'screenshot-ui-show-pointer-button',
                child: iconChild,
                can_focus: true,
                style: 'padding: 8px 16px; min-width: 48px; min-height: 48px;',
            });
            this._sayriButtonClickedId = this._sayriButton.connect('clicked', () => {
                this._handleSayriClick().catch(e => {
                    log(`Shotzy Sayri error: ${e.message}`);
                });
            });
            this._actionsRow.add_child(this._sayriButton);

            const sayriTooltip = new Tooltip(this._sayriButton, 'Ask Sayri');
            ui.add_child(sayriTooltip);
            this._tooltips.push(sayriTooltip);
        }

        // 2. Google Search Button (Colorful Google logo, large size)
        if (!this._lensButton) {
            let googleIconChild;
            const googleSvg = '/usr/share/icons/hicolor/scalable/apps/goa-account-google.svg';
            if (GLib.file_test(googleSvg, GLib.FileTest.EXISTS)) {
                googleIconChild = new St.Icon({
                    gicon: Gio.FileIcon.new(Gio.File.new_for_path(googleSvg)),
                    icon_size: 28,
                });
            } else {
                googleIconChild = new St.Icon({
                    icon_name: 'system-search-symbolic',
                    icon_size: 28,
                });
            }

            this._lensButton = new St.Button({
                style_class: 'screenshot-ui-show-pointer-button',
                child: googleIconChild,
                can_focus: true,
                style: 'padding: 8px 16px; min-width: 48px; min-height: 48px;',
            });
            this._lensButtonClickedId = this._lensButton.connect('clicked', () => {
                this._handleLensClick().catch(e => {
                    log(`Shotzy Search error: ${e.message}`);
                });
            });
            this._actionsRow.add_child(this._lensButton);

            const lensTooltip = new Tooltip(this._lensButton, 'Search with Google');
            ui.add_child(lensTooltip);
            this._tooltips.push(lensTooltip);
        }

        // 3. Copy OCR Text Button (Large size)
        if (!this._copyTextButton) {
            this._copyTextButton = new St.Button({
                style_class: 'screenshot-ui-show-pointer-button',
                child: new St.Icon({
                    icon_name: 'edit-copy-symbolic',
                    icon_size: 28,
                }),
                can_focus: true,
                style: 'padding: 8px 16px; min-width: 48px; min-height: 48px;',
            });
            this._copyTextButtonClickedId = this._copyTextButton.connect('clicked', () => {
                this._ocrController?.copySelectionText();
            });
            this._actionsRow.add_child(this._copyTextButton);

            const copyTooltip = new Tooltip(this._copyTextButton, 'Copy Text');
            ui.add_child(copyTooltip);
            this._tooltips.push(copyTooltip);
        }

        if (this._actionsRow.get_parent() !== ui._panel) {
            ui._panel.add_child(this._actionsRow);
        }

        this._syncActionButtons();
    }

    _syncActionButtons() {
        const ui = Main.screenshotUI;
        if (!ui || !ui._panel)
            return;

        this._ensureButtonsAttached(ui);

        if (this._sayriButton) {
            this._sayriButton.visible = true;
            this._sayriButton.reactive = true;
            this._sayriButton.can_focus = true;
        }

        if (this._lensButton) {
            this._lensButton.visible = true;
            this._lensButton.reactive = true;
            this._lensButton.can_focus = true;
        }

        if (this._copyTextButton) {
            this._copyTextButton.visible = true;
            this._copyTextButton.reactive = true;
            this._copyTextButton.can_focus = true;
        }
    }

    _ensureButtonsAttached(ui) {
        if (this._actionsRow && this._actionsRow.get_parent() !== ui._panel) {
            ui._panel.add_child(this._actionsRow);
        }

        if (this._sayriButton && this._sayriButton.get_parent() !== this._actionsRow)
            this._actionsRow.add_child(this._sayriButton);

        if (this._lensButton && this._lensButton.get_parent() !== this._actionsRow)
            this._actionsRow.add_child(this._lensButton);

        if (this._copyTextButton && this._copyTextButton.get_parent() !== this._actionsRow)
            this._actionsRow.add_child(this._copyTextButton);
    }

    _ensureQrLayout(ui) {
        if (this._lensWrapper)
            return;

        this._lensWrapper = new Clutter.Actor({
            layout_manager: new Clutter.BoxLayout({
                orientation: Clutter.Orientation.HORIZONTAL,
            }),
        });

        this._lensInnerVBox = new Clutter.Actor({
            layout_manager: new Clutter.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
            }),
        });

        this._lensSideBox = new St.BoxLayout({
            vertical: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._lensSideBox.set_style('margin-left: 16px; padding-left: 16px; border-left: 1px solid rgba(255,255,255,0.2);');

        this._lensSideBox.add_child(this._qrButton);

        const typeContainer = ui._typeButtonContainer;
        const bottomContainer = ui._bottomRowContainer;

        if (typeContainer && bottomContainer) {
            ui._panel.remove_child(typeContainer);
            ui._panel.remove_child(bottomContainer);

            this._lensInnerVBox.add_child(typeContainer);
            this._lensInnerVBox.add_child(bottomContainer);

            this._lensWrapper.add_child(this._lensInnerVBox);
            this._lensWrapper.add_child(this._lensSideBox);

            ui._panel.add_child(this._lensWrapper);
        }
    }

    _restoreDefaultLayout(ui) {
        if (!this._lensWrapper)
            return;

        if (ui._panel) {
            ui._panel.remove_child(this._lensWrapper);

            const typeContainer = ui._typeButtonContainer;
            const bottomContainer = ui._bottomRowContainer;

            if (typeContainer && bottomContainer) {
                if (this._lensInnerVBox) {
                    if (typeContainer.get_parent() === this._lensInnerVBox)
                        this._lensInnerVBox.remove_child(typeContainer);
                    if (bottomContainer.get_parent() === this._lensInnerVBox)
                        this._lensInnerVBox.remove_child(bottomContainer);
                }

                ui._panel.add_child(typeContainer);
                ui._panel.add_child(bottomContainer);
            }
        }

        if (this._qrButton && this._lensSideBox && this._qrButton.get_parent() === this._lensSideBox)
            this._lensSideBox.remove_child(this._qrButton);

        this._lensInnerVBox?.destroy();
        this._lensInnerVBox = null;
        this._lensSideBox?.destroy();
        this._lensSideBox = null;
        this._lensWrapper?.destroy();
        this._lensWrapper = null;
    }

    async _handleSayriClick() {
        const ui = Main.screenshotUI;

        const geometry = ui._getSelectedGeometry(true);
        if (!geometry || geometry[2] <= 0 || geometry[3] <= 0) {
            return;
        }
        const [x, y, w, h] = geometry;

        const content = ui._stageScreenshot?.get_content();
        if (!content) return;
        const texture = content.get_texture();

        const stream = Gio.MemoryOutputStream.new_resizable();
        try {
            const pixbuf = await Shell.Screenshot.composite_to_stream(
                texture,
                x, y, w, h,
                ui._scale,
                null, 0, 0, 1,
                stream
            );
            stream.close(null);

            ui.close();

            const filename = GLib.build_filenamev([GLib.get_tmp_dir(), `sayri_circle_${Date.now()}.png`]);
            if (pixbuf.savev(filename, 'png', [], [])) {
                Main.notify('Sayri', 'Opening Sayri with selection...');
                Gio.Subprocess.new(
                    ['python3', '-c', `
import socket, os
from pathlib import Path
candidate_sockets = [
    Path.home() / '.local/share/sayri/sayri.sock',
    Path(f'/run/user/{os.getuid()}/sayri.sock'),
]
for s in candidate_sockets:
    if s.is_socket():
        try:
            c = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            c.connect(str(s))
            c.sendall(b'attach ${filename}\\n')
            c.recv(1024)
            c.close()
            break
        except Exception:
            pass
`],
                    Gio.SubprocessFlags.NONE
                );
            }
        } catch (e) {
            log(`Shotzy Sayri error: ${e.message}`);
            ui.close();
        }
    }

    async _handleLensClick() {
        const ui = Main.screenshotUI;

        const geometry = ui._getSelectedGeometry(true);
        if (!geometry || geometry[2] <= 0 || geometry[3] <= 0) {
            return;
        }
        const [x, y, w, h] = geometry;

        const content = ui._stageScreenshot?.get_content();
        if (!content) return;
        const texture = content.get_texture();

        const stream = Gio.MemoryOutputStream.new_resizable();
        
        try {
            const pixbuf = await Shell.Screenshot.composite_to_stream(
                texture,
                x, y, w, h,
                ui._scale,
                null, 0, 0, 1,
                stream
            );
            stream.close(null);

            ui.close();

            const filename = GLib.build_filenamev([GLib.get_tmp_dir(), `shotzy_${Date.now()}.png`]);
            
            if (pixbuf.savev(filename, 'png', [], [])) {
                Main.notify('Shotzy', 'Uploading screenshot...');
                this._uploader.upload(filename).catch(e => {
                    log(`Shotzy upload error: ${e.message}`);
                    Main.notify('Shotzy', `Upload failed: ${e.message}`);
                });
            } else {
                if (GLib.file_test(filename, GLib.FileTest.EXISTS))
                    GLib.unlink(filename);
                Main.notify('Shotzy', 'Failed to prepare screenshot for upload.');
            }
        } catch (e) {
            log(`Shotzy capture error: ${e.message}`);
            ui.close();
        }
    }

    async _handleQRClick() {
        const ui = Main.screenshotUI;
        if (!ui?._selectionButton?.checked) {
            Main.notify('Shotzy QR', 'Switch to selection mode to scan QR.');
            return;
        }

        if (!GLib.find_program_in_path('zbarimg')) {
            return;
        }

        const geometry = ui._getSelectedGeometry(true);
        if (!geometry || geometry[2] <= 0 || geometry[3] <= 0) {
            return;
        }
        const [x, y, w, h] = geometry;

        const content = ui._stageScreenshot?.get_content();
        if (!content) return;
        const texture = content.get_texture();

        const stream = Gio.MemoryOutputStream.new_resizable();
        const filename = GLib.build_filenamev([GLib.get_tmp_dir(), `shotzy_qr_${Date.now()}.png`]);

        try {
            const pixbuf = await Shell.Screenshot.composite_to_stream(
                texture,
                x, y, w, h,
                ui._scale,
                null, 0, 0, 1,
                stream
            );

            stream.close(null);

            if (pixbuf.savev(filename, 'png', [], [])) {
                const subprocess = Gio.Subprocess.new(
                    ['zbarimg', '--quiet', '--raw', filename],
                    Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
                );

                const [stdout, stderr] = await new Promise((resolve, reject) => {
                    subprocess.communicate_utf8_async(null, null, (proc, res) => {
                        try {
                            const [, out, err] = proc.communicate_utf8_finish(res);
                            resolve([out, err]);
                        } catch (e) {
                            reject(e);
                        }
                    });
                });

                GLib.unlink(filename);

                const result = stdout ? stdout.trim() : null;
                if (result) {
                    // Copy decoded QR content for reuse.
                    const clipboard = St.Clipboard.get_default();
                    clipboard.set_text(St.ClipboardType.CLIPBOARD, result);
                    Main.notify('Shotzy QR', 'Content copied to clipboard.');
                    ui.close();
                } else {
                    this._showScreenshotMessage('No QR code found in selection.');
                }
            } else {
                if (GLib.file_test(filename, GLib.FileTest.EXISTS))
                    GLib.unlink(filename);
                this._showScreenshotMessage('Failed to prepare selection for scanning.');
            }
        } catch (e) {
            log(`Shotzy QR scan error: ${e.message}`);
            if (GLib.file_test(filename, GLib.FileTest.EXISTS))
                GLib.unlink(filename);
            this._showScreenshotMessage(`QR scan failed: ${e.message}`);
        }
    }

    _showScreenshotMessage(text) {
        const ui = Main.screenshotUI;
        if (!ui)
            return;

        if (!this._overlayMessage) {
            this._overlayMessage = new St.Label({
                visible: false,
                opacity: 0,
                reactive: false,
                style: `
                    background-color: rgba(28, 30, 34, 0.96);
                    color: white;
                    padding: 10px 14px;
                    border-radius: 999px;
                    border: 1px solid rgba(255, 255, 255, 0.12);
                    font-weight: 600;
                `,
            });
        }

        if (this._overlayMessage.get_parent() !== ui) {
            this._overlayMessage.get_parent()?.remove_child(this._overlayMessage);
            ui.add_child(this._overlayMessage);
        }

        if (this._overlayMessageTimeoutId) {
            GLib.source_remove(this._overlayMessageTimeoutId);
            this._overlayMessageTimeoutId = 0;
        }

        this._overlayMessage.remove_all_transitions();
        this._overlayMessage.set_text(text);
        this._overlayMessage.opacity = 0;
        this._overlayMessage.show();

        const [, naturalWidth] = this._overlayMessage.get_preferred_width(-1);
        const [, naturalHeight] = this._overlayMessage.get_preferred_height(naturalWidth);
        const extents = ui._panel?.get_transformed_extents();
        const x = Math.max(12, Math.round((global.stage.width - naturalWidth) / 2));
        const y = extents
            ? Math.max(12, Math.round(extents.get_y() - naturalHeight - 16))
            : 24;

        this._overlayMessage.set_position(x, y);
        this._overlayMessage.ease({
            opacity: 255,
            duration: 120,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
        });

        this._overlayMessageTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 1800, () => {
            this._overlayMessageTimeoutId = 0;

            if (!this._overlayMessage)
                return GLib.SOURCE_REMOVE;

            this._overlayMessage.ease({
                opacity: 0,
                duration: 120,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
                onComplete: () => this._overlayMessage?.hide(),
            });

            return GLib.SOURCE_REMOVE;
        });
    }

    _destroyOverlayMessage() {
        if (this._overlayMessageTimeoutId) {
            GLib.source_remove(this._overlayMessageTimeoutId);
            this._overlayMessageTimeoutId = 0;
        }

        if (this._overlayMessage) {
            this._overlayMessage.destroy();
            this._overlayMessage = null;
        }
    }
}
