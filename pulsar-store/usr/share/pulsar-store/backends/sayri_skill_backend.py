#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Sayri AI Skills backend for Pulsar Store."""

import os
import shutil
import zipfile
import tempfile
import urllib.request
from typing import Dict, Any, Optional
from pathlib import Path


class SayriSkillBackend:
    def __init__(self, log_fn=None):
        self.log = log_fn or (lambda msg: None)
        self.skills_dir = os.path.expanduser("~/.config/sayri/skills")
        os.makedirs(self.skills_dir, exist_ok=True)

    def get_skill_path(self, skill_id: str) -> Optional[str]:
        target = os.path.join(self.skills_dir, skill_id)
        if os.path.isdir(target) and os.path.isfile(os.path.join(target, "SKILL.md")):
            return target
        return None

    def is_installed(self, skill_id: str) -> bool:
        return self.get_skill_path(skill_id) is not None

    def get_installed_version(self, skill_id: str) -> Optional[str]:
        skill_path = self.get_skill_path(skill_id)
        if not skill_path:
            return None
        try:
            skill_file = os.path.join(skill_path, "SKILL.md")
            with open(skill_file, "r", encoding="utf-8") as f:
                content = f.read()
            for line in content.splitlines():
                if line.startswith("version:"):
                    return line.split(":", 1)[1].strip().strip('"').strip("'")
            return "1.0.0"
        except Exception:
            return "installed"

    def install(self, skill_id: str, download_url: str = "", raw_content: Optional[str] = None) -> bool:
        self.log(f"[Sayri Skill] Installing skill: {skill_id}...")
        try:
            target_dir = os.path.join(self.skills_dir, skill_id)
            os.makedirs(target_dir, exist_ok=True)
            target_file = os.path.join(target_dir, "SKILL.md")

            if raw_content:
                with open(target_file, "w", encoding="utf-8") as f:
                    f.write(raw_content)
                self.log(f"[Sayri Skill] Skill {skill_id} created successfully.")
                return True

            if not download_url:
                self.log(f"[Sayri Skill] Error: No download URL or content provided for {skill_id}")
                return False

            with tempfile.TemporaryDirectory() as tmpdir:
                tmp_file = os.path.join(tmpdir, "downloaded_skill")
                req = urllib.request.Request(download_url, headers={"User-Agent": "PulsarStore/1.0"})
                with urllib.request.urlopen(req, timeout=20) as resp, open(tmp_file, "wb") as f:
                    shutil.copyfileobj(resp, f)

                # Check if it's a zip file
                if zipfile.is_zipfile(tmp_file):
                    with zipfile.ZipFile(tmp_file, "r") as zf:
                        zf.extractall(target_dir)
                    # If files were in a nested folder, flatten if needed
                    if not os.path.isfile(target_file):
                        for root, _, files in os.walk(target_dir):
                            if "SKILL.md" in files:
                                shutil.copy2(os.path.join(root, "SKILL.md"), target_file)
                                break
                else:
                    # Treat directly as raw markdown file
                    shutil.copy2(tmp_file, target_file)

            self.log(f"[Sayri Skill] Skill {skill_id} installed successfully.")
            return True
        except Exception as e:
            self.log(f"[Sayri Skill] Error installing skill {skill_id}: {e}")
            return False

    def uninstall(self, skill_id: str) -> bool:
        self.log(f"[Sayri Skill] Removing skill: {skill_id}...")
        try:
            target_dir = os.path.join(self.skills_dir, skill_id)
            if os.path.isdir(target_dir):
                shutil.rmtree(target_dir)
            self.log(f"[Sayri Skill] Skill {skill_id} removed successfully.")
            return True
        except Exception as e:
            self.log(f"[Sayri Skill] Error removing skill {skill_id}: {e}")
            return False

    def update(self, skill_id: str, download_url: str = "", raw_content: Optional[str] = None) -> bool:
        return self.install(skill_id, download_url, raw_content)
