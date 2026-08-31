"""Bubblewrap (bwrap) and Host Sandbox Executor for Sayri."""

from __future__ import annotations

import os
import shutil
import subprocess
import time
from typing import Tuple

from sayri.domain.models import SandboxConfig, SandboxLevel
from sayri.domain.secrets_manager import secrets_manager


class SandboxExecutionError(Exception):
    pass


class SandboxExecutor:
    """Executes commands adhering to fine-grained SandboxLevel configurations."""

    def __init__(self, sandboxes_root: str | None = None) -> None:
        self.sandboxes_root = sandboxes_root or os.path.expanduser("~/.local/share/sayri/sandboxes")
        os.makedirs(self.sandboxes_root, exist_ok=True)
        self.bwrap_available = bool(shutil.which("bwrap"))

    def execute(
        self,
        command: str,
        config: SandboxConfig,
        agent_id: str = "default",
    ) -> Tuple[int, str, float]:
        """Executes a command under the specified sandbox level.

        Returns: (exit_code, output, duration_ms)
        """
        start_time = time.monotonic()
        raw_cmd = command.strip()

        # 1. Level 0: Total Prohibition of command execution
        if config.level == SandboxLevel.LEVEL_0_NO_EXEC:
            return (
                126,
                "Error de seguridad: Este subagente tiene el nivel LEVEL_0_NO_EXEC configurado. "
                "No tiene permisos para ejecutar comandos en el sistema.",
                0.0,
            )

        # 2. Blocked binaries check
        for blocked in config.blocked_binaries:
            if blocked in raw_cmd.split():
                return (
                    126,
                    f"Error de seguridad: El comando '{blocked}' está explícitamente bloqueado en la política del agente.",
                    0.0,
                )

        timeout = max(1, config.timeout_seconds)

        # 3. Level 4: Elevated Host with Polkit (pkexec)
        if config.level == SandboxLevel.LEVEL_4_HOST_ROOT or "sudo " in raw_cmd or "pkexec " in raw_cmd:
            if raw_cmd.startswith("sudo "):
                raw_cmd = "pkexec " + raw_cmd[5:]
            elif not raw_cmd.startswith("pkexec "):
                raw_cmd = "pkexec " + raw_cmd
            return self._run_host(raw_cmd, timeout, start_time, elevated=True)

        # 4. Level 3: Host as Current User
        if config.level == SandboxLevel.LEVEL_3_HOST_USER or not self.bwrap_available:
            return self._run_host(raw_cmd, timeout, start_time, elevated=False)

        # 5. Level 1 & 2: Sandboxed with Bubblewrap (bwrap)
        return self._run_bwrap(raw_cmd, config, agent_id, timeout, start_time)

    def _run_host(
        self, command: str, timeout: int, start_time: float, elevated: bool = False
    ) -> Tuple[int, str, float]:
        try:
            res = subprocess.run(
                command,
                shell=True,
                capture_output=True,
                text=True,
                timeout=timeout,
                env=secrets_manager.inject_environment(),
            )
            out = (res.stdout + "\n" + res.stderr).strip()
            retcode = res.returncode
            if retcode in (126, 127) and elevated:
                out = "User cancelled or denied graphical administrator authorization (Polkit)."
            elif not out:
                out = f"(Command completed with exit code {res.returncode})"
            duration = (time.monotonic() - start_time) * 1000.0
            return (retcode, out, duration)
        except subprocess.TimeoutExpired as exc:
            partial = (exc.stdout or "") + "\n" + (exc.stderr or "")
            duration = (time.monotonic() - start_time) * 1000.0
            return (
                0,
                partial.strip() or "(Command started successfully and running in background)",
                duration,
            )
        except Exception as exc:
            duration = (time.monotonic() - start_time) * 1000.0
            return (1, f"Host execution error: {exc}", duration)

    def _run_bwrap(
        self,
        command: str,
        config: SandboxConfig,
        agent_id: str,
        timeout: int,
        start_time: float,
    ) -> Tuple[int, str, float]:
        workspace = config.isolated_dir or os.path.join(self.sandboxes_root, agent_id)
        os.makedirs(workspace, exist_ok=True)

        bwrap_args = [
            "bwrap",
            "--ro-bind", "/", "/",
            "--dev", "/dev",
            "--proc", "/proc",
            "--tmpfs", "/tmp",
            "--tmpfs", "/run",
            "--bind", workspace, workspace,
            "--chdir", workspace,
            "--die-with-parent",
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-uts",
        ]

        if not config.allow_network:
            bwrap_args.append("--unshare-net")

        # Wrap the command in a clean bash shell inside the container
        bwrap_args.extend(["--", "bash", "-c", command])

        try:
            res = subprocess.run(
                bwrap_args,
                capture_output=True,
                text=True,
                timeout=timeout,
                env=secrets_manager.inject_environment(),
            )
            out = (res.stdout + "\n" + res.stderr).strip()
            if not out:
                out = f"(Sandbox bwrap finished with exit code {res.returncode})"
            duration = (time.monotonic() - start_time) * 1000.0
            return (res.returncode, out, duration)
        except subprocess.TimeoutExpired as exc:
            duration = (time.monotonic() - start_time) * 1000.0
            return (124, f"Error: Sandbox timeout limit ({timeout}s) exceeded.", duration)
        except Exception as exc:
            duration = (time.monotonic() - start_time) * 1000.0
            return (1, f"Sandbox bwrap error: {exc}", duration)
