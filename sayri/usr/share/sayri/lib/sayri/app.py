"""Sayri application (GTK4): wires the transparent overlay (orb + cajita),
whisper.cpp STT, OpenAI-compatible LLM and Piper TTS together.

States: idle -> listening -> (activated) -> thinking -> speaking -> listening

A single layer-shell window contains both the Siri orb and the Apple-
intelligence cajita side by side, pinned to the top-right of the monitor.
Clicking the orb toggles the microphone; the cajita handles text input,
reply display and settings/quit buttons.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import threading

import gi

gi.require_version("Gtk", "4.0")

from gi.repository import Gio, GLib, Gtk  # noqa: E402

from . import (  # noqa: E402
    blur_exclusion,
    config,
    llm,
    overlay as overlay_mod,
    paths,
    settings_window,
    stt as stt_mod,
    tts as tts_mod,
)

APP_ID = "es.inled.sayri"
HISTORY_MAX = 10
AUTOSTART_SRC = "/etc/xdg/autostart/sayri.desktop"


def _detect_distro() -> str:
    if os.path.exists("/etc/arch-release"):
        return "Arch Linux"
    elif os.path.exists("/etc/debian_version"):
        return "Debian"
    try:
        with open("/etc/os-release") as f:
            content = f.read().lower()
            if "arch" in content:
                return "Arch Linux"
            elif "debian" in content or "ubuntu" in content:
                return "Debian"
    except Exception:
        pass
    return "Linux"


def markdown_to_plain_speech(text: str) -> str:
    """Clean markdown syntax into natural, clean spoken text for Piper TTS."""
    if not text:
        return ""
    import re
    # Remove code blocks completely so TTS doesn't dictate raw scripts
    cleaned = re.sub(r"```(?:[a-zA-Z0-9_\-]+)?\n?(.*?)\n?```", "", text, flags=re.DOTALL)
    # Convert inline code `foo` -> foo
    cleaned = re.sub(r"`([^`]+)`", r"\1", cleaned)
    # Convert bold / italic
    cleaned = re.sub(r"\*\*([^\*]+)\*\*", r"\1", cleaned)
    cleaned = re.sub(r"__([^_]+)__", r"\1", cleaned)
    cleaned = re.sub(r"(?<!\*)\*(?!\*)([^\*\n]+?)(?<!\*)\*(?!\*)", r"\1", cleaned)
    cleaned = re.sub(r"(?<!\w)_([^\_\n]+?)_(?!\w)", r"\1", cleaned)
    # Convert headers # Header -> Header
    cleaned = re.sub(r"^(?:#{1,6})\s+(.+)$", r"\1", cleaned, flags=re.MULTILINE)
    # Convert list bullets - / *
    cleaned = re.sub(r"^[\*\-]\s+", "", cleaned, flags=re.MULTILINE)
    # Links [text](url) -> text
    cleaned = re.sub(r"\[([^\]]+)\]\([^\)]+\)", r"\1", cleaned)
    # Strip URLs
    cleaned = re.sub(r"https?://\S+", "", cleaned)
    # Remove leftover markdown symbols
    cleaned = re.sub(r"[\*\_~`#><]", "", cleaned)
    cleaned = re.sub(r"\n{2,}", "\n", cleaned).strip()
    return cleaned


def _get_effective_system_prompt(cfg) -> str:
    agent_mode = cfg.get_bool("provider", "agent_mode")
    base_prompt = cfg.get_string("provider", "system_prompt")
    if not agent_mode:
        return base_prompt
    distro = _detect_distro()
    import getpass
    username = getpass.getuser()
    mem_file = paths.memory_file()
    user_f = paths.user_file()
    skills_d = paths.skills_dir()
    return (
        f"Eres Sayri, la asistente inteligente de voz y agente autónomo integrado en Pulsar OS (basado en {distro}).\n"
        f"El usuario actual del sistema es '{username}'. Su perfil y datos están en `{user_f}`.\n"
        f"Tu memoria a largo plazo de recuerdos y preferencias se encuentra en `{mem_file}` (puedes leerla o añadir notas con bash).\n"
        f"Tus habilidades (skills) instaladas de ClawHub/OpenClaw están en `{skills_d}`. Puedes listar tus habilidades con `ls {skills_d}` y leer sus guías con `cat {skills_d}/<skill>/SKILL.md`.\n"
        "Puedes buscar o descargar nuevas habilidades de ClawHub (https://clawhub.ai) usando el comando `sayri-skills install <skill-name>` o `sayri-skills search <query>`.\n"
        "Para capturar la pantalla, puedes usar el comando `sayri screenshot [ruta_destino]` o portales de GNOME.\n"
        "Para mover el cursor o automatizar clics/teclado en Wayland/X11, dispones de `ydotool` (`ydotool mousemove -a X Y`, `ydotool click 0xC0`) y `xdotool`.\n\n"
        "FLUJO AGÉNTICO AUTÓNOMO:\n"
        "Cuando el usuario te pida una tarea, abrir aplicaciones, modificar ajustes o consultar datos, emite un bloque:\n"
        "```bash\n"
        "<comando bash>\n"
        "```\n"
        "Ejecutarás comandos de forma interactiva. Si un comando falla o necesitas más pasos, recibirás el código de salida y error en el siguiente turno y podrás emitir nuevos comandos bash para corregir el error hasta lograr el objetivo.\n"
        "Responde siempre de forma natural, concisa y agradable (1 a 3 frases habladas para voz)."
    )


class SayriApp(Gtk.Application):
    def __init__(self) -> None:
        super().__init__(application_id=APP_ID,
                         flags=Gio.ApplicationFlags.NON_UNIQUE)
        self.cfg = config.config

        self.stt = stt_mod.STTEngine(self.cfg)
        self.tts = tts_mod.TTSEngine(self.cfg)

        self.overlay: overlay_mod.SayriOverlay | None = None
        self.settings_win = None

        self.state = "idle"
        self.armed = False
        self._busy = False
        self._mic_on = False
        self.session = None
        self._assistant_text = ""
        self.history: list[tuple[str, str]] = []

        self.cfg.on_change(self._on_config_change)
        self.connect("activate", self._on_activate)
        self.connect("shutdown", self._on_shutdown)
        self.hold()

    # ── public state helpers
    @property
    def busy(self) -> bool:
        return self._busy

    def listening_now(self) -> bool:
        return bool(self._mic_on)

    # ── startup
    def _on_activate(self, _app) -> None:
        paths.ensure_dirs()
        blur_exclusion.apply_blur_exclusion()
        if self.overlay is None:
            self._build_ui()
        self.overlay.show()
        mode = self.cfg.get_string("stt", "mode")
        if mode in ("always", "wakeword"):
            self._start_session()
        self.refresh_status()
        self._start_ipc_server()
        self._launch_indicator()

    def _launch_indicator(self) -> None:
        if hasattr(self, "_indicator_proc") and self._indicator_proc and self._indicator_proc.poll() is None:
            return
        try:
            env = dict(os.environ)
            lib_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
            cur = env.get("PYTHONPATH", "")
            env["PYTHONPATH"] = f"{lib_dir}:{cur}".rstrip(":")
            env.pop("LD_PRELOAD", None)
            self._indicator_proc = subprocess.Popen([sys.executable, "-m", "sayri.indicator"], env=env)
            print(f"[Sayri] 🚀 Launched tray indicator (PID: {self._indicator_proc.pid})")
        except Exception as exc:
            print(f"[Sayri] Indicator launch error: {exc}")

    def _start_ipc_server(self) -> None:
        sock_path = os.path.join(paths.state_dir(), "sayri.sock")
        if os.path.exists(sock_path):
            try:
                os.remove(sock_path)
            except OSError:
                pass

        def _worker() -> None:
            try:
                import socket
                self._ipc_sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                self._ipc_sock.bind(sock_path)
                self._ipc_sock.listen(5)
                while getattr(self, "_ipc_running", False):
                    try:
                        conn, _ = self._ipc_sock.accept()
                        data = conn.recv(1024).decode("utf-8").strip()
                        if data == "toggle":
                            GLib.idle_add(self.toggle_visible)
                        elif data == "show":
                            GLib.idle_add(lambda: self.overlay.show() if self.overlay else None)
                        elif data == "hide":
                            GLib.idle_add(lambda: self.overlay.hide() if self.overlay else None)
                        elif data == "listen":
                            GLib.idle_add(self.start_listening)
                        elif data == "settings":
                            GLib.idle_add(self.open_settings)
                        elif data == "quit":
                            GLib.idle_add(self.quit_app)
                        conn.sendall(b"OK\n")
                        conn.close()
                    except Exception:
                        break
            except Exception as exc:
                print(f"[Sayri] IPC server: {exc}")

        self._ipc_running = True
        threading.Thread(target=_worker, daemon=True).start()

    def toggle_visible(self) -> None:
        if self.overlay:
            self.overlay.toggle()

    def _build_ui(self) -> None:
        self.overlay = overlay_mod.SayriOverlay(self)

    # ── state
    def set_state(self, state: str) -> None:
        self.state = state
        if self.overlay:
            self.overlay.set_state_sync(state)

    # ── orb events
    def on_orb_click(self) -> None:
        """The orb toggles the microphone on/off."""
        self.toggle_listening()

    # ── bridge to the cajita (via overlay)
    def _msg(self, kind: str, text: str) -> None:
        if self.overlay:
            self.overlay.set_content(kind, text)

    def _set_partial(self, text: str) -> None:
        if self.overlay:
            self.overlay.set_content("partial", text)

    def _set_assistant(self, text: str) -> None:
        if self.overlay:
            self.overlay.set_content("assistant", text)

    def _set_mic(self, active: bool) -> None:
        self._mic_on = bool(active)
        if self.overlay:
            self.overlay.set_mic(self._mic_on)

    def _set_busy(self, busy: bool) -> None:
        self._busy = busy
        if self.overlay:
            self.overlay.set_busy(busy)

    # ── STT
    def _start_session(self) -> None:
        if self.session and self.session.is_running():
            return
        if not self.stt.ready:
            self._msg("hint",
                      "Missing: " + ", ".join(self.stt.missing())
                      + ". Download in Settings → Speech Recognition.")
            return
        self.session = self.stt.create_session(
            on_partial=self._on_partial,
            on_utterance=self._on_utterance,
            on_level=self._on_level,
            on_speech_start=lambda: GLib.idle_add(self._on_speech_start),
            on_transcribe_start=lambda: GLib.idle_add(self._on_transcribe_start),
        )
        if self.session.start():
            self.set_state("listening")
            self._set_mic(True)
        else:
            self.session = None
            self._msg("error", "Could not initialize microphone.")

    def _stop_session(self) -> None:
        if self.session:
            self.session.stop()
            self.session = None
        self._set_mic(False)

    def start_listening(self) -> None:
        if self._busy:
            return
        self.armed = True
        if not (self.session and self.session.is_running()):
            self._start_session()
        self.set_state("activated")
        self._msg("hint", "Listening…")

    def stop_listening(self) -> None:
        self.armed = False
        self._stop_session()
        if not self._busy:
            self.set_state("idle")

    def toggle_listening(self) -> None:
        if self.session and self.session.is_running():
            if self.armed:
                self.session.flush()
                self.armed = False
                self.set_state("idle")
            else:
                self.armed = True
                self.set_state("activated")
                self._msg("hint", "Listening…")
        else:
            self.start_listening()

    def _apply_mode(self) -> None:
        self._stop_session()
        mode = self.cfg.get_string("stt", "mode")
        if mode in ("always", "wakeword"):
            self._start_session()
        else:
            self.set_state("idle")

    # ── STT callbacks (threads)
    def _on_speech_start(self) -> None:
        if self._busy:
            return
        self.set_state("listening")
        self._msg("hint", "Listening…")

    def _on_transcribe_start(self) -> None:
        self._stop_session()
        if self._busy:
            return
        self.set_state("thinking")
        self._msg("hint", "Transcribing…")

    def _on_partial(self, text: str) -> None:
        GLib.idle_add(self._handle_partial, text)

    def _on_utterance(self, text: str) -> None:
        GLib.idle_add(self._handle_utterance, text)

    def _on_level(self, level: float) -> None:
        if self.overlay:
            self.overlay.set_audio_level(level)

    def _handle_partial(self, text: str) -> None:
        if self._busy:
            return
        self._set_partial(f"“{text}…”")

    def _handle_utterance(self, text: str) -> None:
        if not text.strip():
            self._set_partial("")
            mode = self.cfg.get_string("stt", "mode")
            if mode == "manual" or not self.armed:
                self.set_state("idle")
                self._msg("hint", "Ask me anything…")
            return
        if self._busy:
            self._msg("hint", "Please wait for the response to finish.")
            return

        # Check if text is self-echo from the last assistant response
        if getattr(self, "_last_assistant_reply", None):
            prev_clean = re.sub(r"[^\w\s]", "", self._last_assistant_reply.lower())
            curr_clean = re.sub(r"[^\w\s]", "", text.lower())
            if len(curr_clean) > 3 and (curr_clean in prev_clean or (len(prev_clean) > 3 and prev_clean in curr_clean)):
                print(f"[Sayri] ℹ️ Ignored TTS self-echo: \"{text}\"")
                return

        mode = self.cfg.get_string("stt", "mode")
        matched, remainder = self._match_and_extract_wake_word(text)
        ui_open = bool(self.overlay and self.overlay.is_visible)

        # In wakeword mode while running in background (UI closed and not armed), require wake word
        if mode == "wakeword" and not self.armed and not ui_open:
            if matched:
                if self.overlay and not self.overlay.is_visible:
                    self.overlay.show()
                if remainder and len(remainder) > 1:
                    # User asked the question in the same sentence as the wake word
                    self.armed = False
                    self.send_text(remainder)
                else:
                    # User only said the wake word
                    self.armed = True
                    self.set_state("activated")
                    self._msg("hint", "Listening…")
                return
            else:
                print(f"[Sayri] ℹ️ Wake word not found in \"{text}\" (mode=wakeword, UI hidden)")
                return

        # If UI is showing or armed, user can say anything!
        # If user spoke ONLY the wake word without a question, arm and wait
        if matched and (not remainder or len(remainder) <= 1):
            if self.overlay and not self.overlay.is_visible:
                self.overlay.show()
            self.armed = True
            self.set_state("activated")
            self._msg("hint", "Listening…")
            print("[Sayri] ℹ️ Wake word detected without question, waiting for prompt...")
            return

        if self.overlay and not self.overlay.is_visible:
            self.overlay.show()

        # If matched wake word with a query, send the query without the wake word; otherwise send full text
        query = remainder if (matched and remainder and len(remainder) > 1) else text
        self.armed = False
        self.send_text(query)
        if mode == "manual":
            self._stop_session()
            self.set_state("idle")

    def _match_and_extract_wake_word(self, text: str) -> tuple[bool, str]:
        raw = text.strip()
        import re

        cfg_ww = self.cfg.get_string("stt", "wake_word").strip().lower()
        candidates = set()
        if cfg_ww:
            for item in cfg_ww.split(","):
                clean = item.strip().lower()
                if clean:
                    candidates.add(clean)

        # Common phonetic variants of Sayri / Siri
        candidates.update([
            "hey sayri", "oye sayri", "sayri", "hola sayri",
            "hey sairi", "oye sairi", "sairi", "hola sairi",
            "hey sari", "oye sari", "sari", "hola sari",
            "hey seiri", "oye seiri", "seiri",
            "hey seyri", "oye seyri", "seyri",
            "hey siri", "oye siri", "siri", "hola siri",
            "hey sara", "oye sara", "sara",
        ])

        # Convert spaces to flexible \s+
        regex_parts = []
        for w in sorted(candidates, key=len, reverse=True):
            parts = [re.escape(p) for p in w.split()]
            regex_parts.append(r"\s+".join(parts))

        full_regex = re.compile(r"\b(?:" + "|".join(regex_parts) + r")\b", re.IGNORECASE)

        # Replace punctuation with spaces for matching
        cleaned = re.sub(r"[,;:\.¿\?¡!\-_]", " ", raw)
        m = full_regex.search(cleaned)
        if m:
            end_pos = m.end()
            remainder = raw[end_pos:].strip(" \t\n\r,:;¿?¡!.")
            matched_word = m.group(0).strip()
            print(f"[Sayri] 🎯 Wake word '{matched_word}' matched in \"{raw}\" -> command: \"{remainder}\"")
            return True, remainder

        return False, ""

    # ── LLM
    def send_text(self, text: str) -> None:
        text = text.strip()
        if not text:
            return
        self._stop_session()
        self.tts.cancel()
        self.armed = False
        if self.overlay:
            self.overlay.clear()
        self._assistant_text = ""
        self._set_busy(True)
        self.set_state("thinking")
        self._msg("user", text)
        print(f"[Sayri] 🤖 Querying AI ({self.cfg.get_string('provider', 'model')}): \"{text}\"")

        messages = [{
            "role": "system",
            "content": _get_effective_system_prompt(self.cfg),
        }]
        for role, content in self.history[-HISTORY_MAX:]:
            messages.append({"role": role, "content": content})
        messages.append({"role": "user", "content": text})
        self.history.append(("user", text))

        threading.Thread(target=self._llm_worker, args=(messages, 1),
                         daemon=True).start()

    def _llm_worker(self, messages: list[dict], depth: int = 1) -> None:
        llm.stream_chat(
            self.cfg.get_string("provider", "base_url"),
            self.cfg.get_string("provider", "api_key"),
            self.cfg.get_string("provider", "model"),
            messages,
            temperature=self.cfg.get_float("provider", "temperature"),
            max_tokens=self.cfg.get_int("provider", "max_tokens") or None,
            stream=self.cfg.get_bool("provider", "stream"),
            timeout=self.cfg.get_int("provider", "timeout"),
            on_delta=lambda d: GLib.idle_add(self._on_delta, d),
            on_done=lambda full: GLib.idle_add(self._on_done, full, messages, depth),
            on_error=lambda e: GLib.idle_add(self._on_error, e),
        )

    def _on_delta(self, delta: str) -> None:
        self._assistant_text += delta
        self._set_assistant(self._assistant_text)

    def _on_done(self, full: str, messages: list[dict] | None = None, depth: int = 1) -> None:
        import re
        if full and self.cfg.get_bool("provider", "agent_mode") and messages and depth < 6:
            m = re.search(r"```(?:bash|sh)?\s*\n(.*?)\n```", full, re.DOTALL)
            if m:
                cmd = m.group(1).strip()
                if cmd:
                    print(f"[Sayri] ⚙️ Agent step {depth} executing: `{cmd}`")
                    self._msg("hint", f"⚙️ Ejecutando ({depth}): {cmd[:36]}…")
                    threading.Thread(target=self._execute_tool_and_followup,
                                     args=(messages, full, cmd, depth), daemon=True).start()
                    return

        if full:
            self._assistant_text = full
            self._set_assistant(full)
            self.history.append(("assistant", full))
            self.history = self.history[-HISTORY_MAX * 2:]
        self._finish_reply(full)

    def _execute_tool_and_followup(self, messages: list[dict], full_reply: str, cmd: str, depth: int = 1) -> None:
        retcode = 0
        try:
            res = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=45)
            output = (res.stdout + "\n" + res.stderr).strip()
            retcode = res.returncode
            if not output:
                output = f"(Command exited with code {res.returncode})"
        except Exception as exc:
            output = f"Command error: {exc}"
            retcode = 1

        print(f"[Sayri] ✓ Tool step {depth} exit {retcode} ({len(output)} bytes): \"{output[:100]}...\"")
        followup_messages = list(messages)
        followup_messages.append({"role": "assistant", "content": full_reply})

        if retcode != 0:
            prompt_content = (
                f"[Tool Error - Exit code {retcode}]:\nCommand: `{cmd}`\nOutput:\n{output}\n\n"
                "The command failed. Analyze what went wrong, fix the command and emit a new ```bash ... ``` block to retry, or explain the error."
            )
        else:
            prompt_content = (
                f"[Tool Output - Exit code 0]:\nCommand: `{cmd}`\nOutput:\n{output}\n\n"
                "If you need another command, emit a ```bash ... ``` block. Otherwise, summarize the final result naturally for voice in 1-2 sentences."
            )

        followup_messages.append({
            "role": "user",
            "content": prompt_content,
        })

        GLib.idle_add(lambda: self.overlay and self.overlay.cajita.set_tool_output(cmd, output))
        self._assistant_text = ""
        self._llm_worker(followup_messages, depth + 1)

    def _on_error(self, exc: Exception) -> None:
        self._msg("error", f"Provider error: {exc}")
        self._set_busy(False)
        if self.overlay:
            self.overlay.cajita.set_speaking(False)
        self._after_reply()

    def _finish_reply(self, full: str) -> None:
        self._last_assistant_reply = full
        spoken = markdown_to_plain_speech(full)
        if self.cfg.get_bool("tts", "enabled") and spoken and self.tts.ready:
            # Stop microphone during TTS speech to prevent feedback loop!
            self._stop_session()
            self.set_state("speaking")
            print(f"[Sayri] 🔊 Speaking response with Piper TTS: \"{spoken[:60]}...\"")
            self.tts.speak_async(
                spoken,
                on_level=lambda lvl: GLib.idle_add(self._on_level, lvl),
                on_end=lambda: GLib.idle_add(self._after_reply),
                on_error=lambda e: GLib.idle_add(self._on_error, e),
            )
        else:
            self._after_reply()

    def _after_reply(self) -> None:
        self._set_busy(False)
        self._on_level(0.0)
        self._assistant_text = ""
        if self.overlay:
            self.overlay.cajita.set_speaking(False)
        mode = self.cfg.get_string("stt", "mode")
        if mode in ("always", "wakeword"):
            self._start_session()
            if mode == "wakeword":
                self.set_state("idle")
            else:
                self.set_state("listening")
        else:
            self.set_state("idle")

    # ── UI glue
    def toggle_visible(self) -> None:
        if self.overlay:
            self.overlay.toggle()

    def open_settings(self) -> None:
        import subprocess
        import sys
        try:
            if hasattr(self, "_settings_proc") and self._settings_proc and self._settings_proc.poll() is None:
                return
            env = dict(os.environ)
            lib_path = os.path.dirname(os.path.dirname(__file__))
            env["PYTHONPATH"] = lib_path + (":" + env["PYTHONPATH"] if "PYTHONPATH" in env else "")
            env.pop("LD_PRELOAD", None)
            self._settings_proc = subprocess.Popen([sys.executable, "-m", "sayri.settings_gtk3"], env=env)
        except Exception:
            if self.settings_win is None:
                self.settings_win = settings_window.SettingsWindow(self)
            self.settings_win.show()

    def refresh_status(self) -> None:
        if self.settings_win is not None:
            self.settings_win.refresh_status()

    def apply_ui_config(self) -> None:
        if self.overlay:
            self.overlay.apply_config()

    def apply_autostart(self) -> None:
        autostart_dir = os.path.expanduser("~/.config/autostart")
        dest = os.path.join(autostart_dir, "sayri.desktop")
        if self.cfg.get_bool("ui", "autostart"):
            try:
                os.makedirs(autostart_dir, exist_ok=True)
                shutil.copy2(AUTOSTART_SRC, dest)
            except OSError as exc:
                print(f"[sayri] autostart: {exc}")
        else:
            try:
                os.remove(dest)
            except OSError:
                pass

    def _on_config_change(self, group: str, key: str, _value) -> None:
        if group == "ui":
            self.apply_ui_config()
        elif group == "stt" and key == "mode":
            self._apply_mode()

    # ── shutdown
    def quit_app(self) -> None:
        self._ipc_running = False
        if hasattr(self, "_indicator_proc") and self._indicator_proc and self._indicator_proc.poll() is None:
            try:
                self._indicator_proc.terminate()
            except Exception:
                pass
        sock_path = os.path.join(paths.state_dir(), "sayri.sock")
        if os.path.exists(sock_path):
            try:
                os.remove(sock_path)
            except OSError:
                pass
        self._stop_session()
        self.tts.cancel()
        self.quit()

    def _on_shutdown(self, _app) -> None:
        self._stop_session()
        self.tts.cancel()


def main() -> int:
    # If an instance is already running, toggle it and exit
    sock_path = os.path.join(paths.state_dir(), "sayri.sock")
    if os.path.exists(sock_path):
        try:
            import socket
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.settimeout(0.5)
            s.connect(sock_path)
            s.sendall(b"toggle\n")
            s.recv(1024)
            s.close()
            print("[Sayri] Toggled running instance via IPC.")
            return 0
        except Exception:
            pass

    app = SayriApp()
    app.hold()
    if os.environ.get("SAYRI_AUTOQUIT_MS"):
        try:
            GLib.timeout_add(int(os.environ["SAYRI_AUTOQUIT_MS"]), app.quit)
        except ValueError:
            pass
    code = app.run(sys.argv)
    print(f"[sayri] run() terminó con código {code}")
    return code
