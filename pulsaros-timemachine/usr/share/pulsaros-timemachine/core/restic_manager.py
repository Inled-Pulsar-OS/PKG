#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pulsar OS Time Machine - Restic Backup Engine
Wraps Restic CLI operations with progress streaming, JSON parsing, cancellation, and manual snapshot deletion.
"""

import os
import sys
import subprocess
import json
import shutil
import threading
from typing import Dict, List, Optional, Tuple, Callable, Any

class ResticManager:
    """Manages Restic repository operations: init, backup, snapshots, restore, forget, prune, cancel, delete."""

    def __init__(self, repo_url: str, password: str, rclone_config: Optional[str] = None):
        self.repo_url = repo_url
        self.password = password
        self.rclone_config = rclone_config
        self.current_process: Optional[subprocess.Popen] = None
        self._cancel_requested = False

    def _get_env(self) -> Dict[str, str]:
        env = os.environ.copy()
        env["RESTIC_PASSWORD"] = self.password
        if self.rclone_config and os.path.exists(self.rclone_config):
            env["RCLONE_CONFIG"] = self.rclone_config
        return env

    def _build_cmd(self, subcmd: str, *args) -> List[str]:
        cmd = ["restic", "-r", self.repo_url, subcmd]
        cmd.extend(args)
        return cmd

    def is_repo_initialized(self) -> bool:
        """Checks if the restic repository exists and can be accessed with password."""
        cmd = self._build_cmd("snapshots", "--json")
        try:
            res = subprocess.run(
                cmd,
                env=self._get_env(),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=20
            )
            return res.returncode == 0
        except Exception:
            return False

    def init_repo(self) -> Tuple[bool, str]:
        """Initializes a new Restic repository."""
        cmd = self._build_cmd("init")
        try:
            res = subprocess.run(
                cmd,
                env=self._get_env(),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=60
            )
            if res.returncode == 0:
                return True, "Repository successfully initialized."
            else:
                return False, f"Failed to initialize repository: {res.stderr.strip()}"
        except Exception as e:
            return False, f"Init error: {e}"

    def unlock(self) -> bool:
        """Unlocks a stale locked repository."""
        cmd = self._build_cmd("unlock")
        try:
            res = subprocess.run(cmd, env=self._get_env(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            return res.returncode == 0
        except Exception:
            return False

    def cancel_backup(self) -> bool:
        """Cancels the currently running backup process."""
        self._cancel_requested = True
        if self.current_process and self.current_process.poll() is None:
            try:
                self.current_process.terminate()
                self.current_process.wait(timeout=3)
            except Exception:
                try:
                    self.current_process.kill()
                except Exception:
                    pass
            self.unlock()
            return True
        return False

    def list_snapshots(self) -> List[Dict[str, Any]]:
        """
        Returns a parsed list of all snapshots in the repository with metadata.
        """
        cmd = self._build_cmd("snapshots", "--json")
        try:
            res = subprocess.run(
                cmd,
                env=self._get_env(),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=30
            )
            if res.returncode == 0 and res.stdout.strip():
                data = json.loads(res.stdout)
                return sorted(data, key=lambda s: s.get("time", ""), reverse=True)
            return []
        except Exception as e:
            print(f"[ResticManager] Error listing snapshots: {e}")
            return []

    def run_backup(
        self,
        paths_to_backup: List[str],
        tags: Optional[List[str]] = None,
        excludes: Optional[List[str]] = None,
        progress_callback: Optional[Callable[[Dict[str, Any]], None]] = None,
        log_callback: Optional[Callable[[str], None]] = None
    ) -> Tuple[bool, str, Dict[str, Any]]:
        """
        Runs a backup of the specified paths.
        Streams JSON status messages via progress_callback and log lines via log_callback.
        """
        self.unlock()
        self._cancel_requested = False

        args = ["--json"]
        if tags:
            for t in tags:
                args.extend(["--tag", t])
        if excludes:
            for exc in excludes:
                exc_clean = exc.strip()
                if exc_clean:
                    args.extend(["--exclude", exc_clean])
        args.extend(paths_to_backup)

        cmd = self._build_cmd("backup", *args)
        stats = {}
        error_lines = []

        if log_callback:
            log_callback(f"Starting restic backup: {' '.join(paths_to_backup)}")

        try:
            self.current_process = subprocess.Popen(
                cmd,
                env=self._get_env(),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1
            )

            def read_stderr():
                for line in self.current_process.stderr:
                    clean = line.strip()
                    if clean:
                        error_lines.append(clean)
                        if log_callback:
                            log_callback(f"[ERR] {clean}")

            err_thread = threading.Thread(target=read_stderr)
            err_thread.daemon = True
            err_thread.start()

            for line in self.current_process.stdout:
                line_str = line.strip()
                if not line_str:
                    continue
                try:
                    event = json.loads(line_str)
                    msg_type = event.get("message_type")
                    if msg_type == "status" and progress_callback:
                        progress_callback(event)
                    elif msg_type == "summary":
                        stats = event
                        if log_callback:
                            files_new = event.get("files_new", 0)
                            bytes_added = event.get("data_added", 0) / (1024 * 1024)
                            log_callback(f"Backup summary: {files_new} new files, {bytes_added:.2f} MB added.")
                except json.JSONDecodeError:
                    if log_callback:
                        log_callback(line_str)

            self.current_process.wait()
            err_thread.join(timeout=2)

            if self._cancel_requested:
                self.unlock()
                return False, "Backup was cancelled by user.", stats

            if self.current_process.returncode == 0:
                snap_id = stats.get("snapshot_id", "unknown")
                return True, f"Backup completed successfully (Snapshot: {snap_id[:8]})", stats
            else:
                err_msg = "\n".join(error_lines) or "Unknown error during backup."
                return False, f"Backup failed: {err_msg}", stats

        except Exception as e:
            if self._cancel_requested:
                return False, "Backup cancelled.", stats
            return False, f"Backup exception: {e}", stats
        finally:
            self.current_process = None

    def delete_snapshot(self, snapshot_id: str, log_callback: Optional[Callable[[str], None]] = None) -> Tuple[bool, str]:
        """
        Manually deletes a specific snapshot from the repository and prunes freed blocks.
        """
        self.unlock()
        cmd = self._build_cmd("forget", snapshot_id, "--prune")
        if log_callback:
            log_callback(f"Deleting snapshot {snapshot_id} from repository...")

        try:
            res = subprocess.run(
                cmd,
                env=self._get_env(),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=180
            )
            if res.returncode == 0:
                if log_callback:
                    log_callback(f"Snapshot {snapshot_id[:8]} deleted and repository pruned.")
                return True, f"Snapshot {snapshot_id[:8]} deleted successfully."
            else:
                return False, f"Failed to delete snapshot: {res.stderr.strip()}"
        except Exception as e:
            return False, f"Error deleting snapshot: {e}"

    def forget_and_prune(self, keep_last: int = 10, log_callback: Optional[Callable[[str], None]] = None) -> Tuple[bool, str]:
        """
        Prunes older snapshots keeping the specified number of most recent snapshots.
        """
        self.unlock()
        cmd = self._build_cmd("forget", "--keep-last", str(keep_last), "--prune")
        if log_callback:
            log_callback(f"Running prune policy (keeping last {keep_last} snapshots)...")

        try:
            res = subprocess.run(
                cmd,
                env=self._get_env(),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=180
            )
            if res.returncode == 0:
                if log_callback:
                    log_callback("Retention prune completed.")
                return True, "Retention pruning completed successfully."
            else:
                return False, f"Prune warning: {res.stderr.strip()}"
        except Exception as e:
            return False, f"Prune error: {e}"

    def restore_snapshot(
        self,
        snapshot_id: str,
        target_dir: str,
        include_path: Optional[str] = None,
        progress_callback: Optional[Callable[[str], None]] = None,
        log_callback: Optional[Callable[[str], None]] = None
    ) -> Tuple[bool, str]:
        """
        Restores files from a snapshot into target_dir.
        """
        self.unlock()
        os.makedirs(target_dir, exist_ok=True)
        
        args = ["--target", target_dir]
        if include_path:
            args.extend(["--include", include_path])
        args.append(snapshot_id)

        cmd = self._build_cmd("restore", *args)
        if log_callback:
            log_callback(f"Restoring snapshot {snapshot_id} to {target_dir}...")

        try:
            process = subprocess.Popen(
                cmd,
                env=self._get_env(),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )

            for line in process.stdout:
                line_str = line.strip()
                if line_str and log_callback:
                    log_callback(line_str)

            process.wait()
            if process.returncode == 0:
                return True, f"Snapshot {snapshot_id[:8]} restored successfully to {target_dir}."
            else:
                err = process.stderr.read()
                return False, f"Restore failed: {err.strip()}"
        except Exception as e:
            return False, f"Restore error: {e}"

    def list_snapshot_files(self, snapshot_id: str) -> List[Dict[str, Any]]:
        """Lists files inside a given snapshot."""
        cmd = self._build_cmd("ls", snapshot_id, "--json")
        try:
            res = subprocess.run(
                cmd,
                env=self._get_env(),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=30
            )
            if res.returncode == 0:
                files = []
                for line in res.stdout.strip().split("\n"):
                    if line:
                        try:
                            files.append(json.loads(line))
                        except Exception:
                            pass
                return files
            return []
        except Exception:
            return []
