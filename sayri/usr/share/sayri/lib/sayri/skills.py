"""ClawHub / OpenClaw Skills Manager for Sayri.

Allows Sayri to discover, download, list, and read skills from ClawHub
(https://clawhub.ai) and the OpenClaw ecosystem into ~/.config/sayri/skills/.
"""

from __future__ import annotations

import json
import os
import shutil
import sys
import urllib.error
import urllib.parse
import urllib.request
from typing import Any, Optional

from . import paths

CLAWHUB_API_BASE = "https://clawhub.ai/api/v1"
CLAWHUB_RAW_BASE = "https://raw.githubusercontent.com/openclaw/skills/main"


def list_skills() -> list[dict[str, Any]]:
    """List all locally installed skills in ~/.config/sayri/skills/."""
    skills_root = paths.skills_dir()
    if not os.path.isdir(skills_root):
        return []

    result = []
    for entry in sorted(os.listdir(skills_root)):
        entry_path = os.path.join(skills_root, entry)
        if not os.path.isdir(entry_path):
            continue
        skill_file = os.path.join(entry_path, "SKILL.md")
        desc = ""
        if os.path.isfile(skill_file):
            try:
                with open(skill_file, "r", encoding="utf-8") as f:
                    for line in f:
                        line_s = line.strip()
                        if line_s.startswith("description:"):
                            desc = line_s.split("description:", 1)[1].strip(" \"'")
                            break
                        elif line_s and not line_s.startswith(("#", "---", "name:")) and not desc:
                            desc = line_s
            except Exception:
                pass
        result.append({
            "name": entry,
            "path": entry_path,
            "skill_file": skill_file if os.path.isfile(skill_file) else None,
            "description": desc or "Installed skill",
        })
    return result


def read_skill(name: str) -> Optional[str]:
    """Read the full SKILL.md of an installed skill."""
    skill_file = os.path.join(paths.skills_dir(), name, "SKILL.md")
    if os.path.isfile(skill_file):
        try:
            with open(skill_file, "r", encoding="utf-8") as f:
                return f.read()
        except Exception as exc:
            return f"Error reading skill {name}: {exc}"
    return None


def search_clawhub(query: str) -> list[dict[str, Any]]:
    """Search ClawHub registry (https://clawhub.ai) for skills."""
    url = f"{CLAWHUB_API_BASE}/search?q={urllib.parse.quote(query)}"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Sayri-Pulsar/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            results = data.get("results", data.get("skills", []))
            out = []
            for r in results:
                out.append({
                    "slug": r.get("slug") or r.get("name"),
                    "name": r.get("displayName") or r.get("name") or r.get("slug"),
                    "owner": r.get("ownerHandle") or (r.get("owner") or {}).get("handle", ""),
                    "description": r.get("summary") or r.get("description") or "",
                    "downloads": r.get("downloads", 0),
                })
            return out
    except Exception as exc:
        print(f"[Skills] ClawHub search warning: {exc}")
        return []


def install_skill(slug_or_name: str) -> bool:
    """Download and extract official skill package from ClawHub into ~/.config/sayri/skills/<name>/."""
    import io
    import zipfile

    slug_raw = slug_or_name.strip().lstrip("@")
    owner = None
    slug = slug_raw
    if "/" in slug_raw:
        owner, slug = slug_raw.split("/", 1)

    # If owner not specified, search ClawHub to find top matching owner
    if not owner:
        search_res = search_clawhub(slug)
        if search_res:
            owner = search_res[0].get("owner")
            if search_res[0].get("slug"):
                slug = search_res[0]["slug"]

    target_dir = os.path.join(paths.skills_dir(), slug)
    os.makedirs(target_dir, exist_ok=True)

    # 1. Download official ZIP from ClawHub API
    dl_url = f"{CLAWHUB_API_BASE}/download?slug={slug}" + (f"&ownerHandle={owner}" if owner else "")
    print(f"[Skills] 🌐 Fetching from ClawHub: {dl_url}")

    try:
        req = urllib.request.Request(dl_url, headers={"User-Agent": "Sayri-Pulsar/1.0"})
        with urllib.request.urlopen(req, timeout=15) as resp:
            content = resp.read()
            if content.startswith(b"PK\x03\x04") or b"SKILL.md" in content:
                with zipfile.ZipFile(io.BytesIO(content)) as z:
                    z.extractall(target_dir)
                print(f"[Skills] ✓ Extracted official ClawHub skill '{slug}' into {target_dir}")
                return True
            elif content.startswith(b"{"):
                # JSON redirect with archiveUrl
                meta = json.loads(content.decode("utf-8"))
                if meta.get("archiveUrl"):
                    arc_req = urllib.request.Request(meta["archiveUrl"], headers={"User-Agent": "Sayri-Pulsar/1.0"})
                    with urllib.request.urlopen(arc_req, timeout=20) as a_resp:
                        with zipfile.ZipFile(io.BytesIO(a_resp.read())) as z:
                            z.extractall(target_dir)
                    print(f"[Skills] ✓ Downloaded and extracted '{slug}' from archiveUrl")
                    return True
    except Exception as exc:
        print(f"[Skills] Warning: ClawHub download error ({exc})")

    # 2. Fallback to GitHub OpenClaw skills repo
    github_urls = [
        f"{CLAWHUB_RAW_BASE}/skills/{slug}/SKILL.md",
        f"{CLAWHUB_RAW_BASE}/{slug}/SKILL.md",
    ]
    for g_url in github_urls:
        try:
            req = urllib.request.Request(g_url, headers={"User-Agent": "Sayri-Pulsar/1.0"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = resp.read().decode("utf-8")
                if data and "404" not in data[:40]:
                    target_file = os.path.join(target_dir, "SKILL.md")
                    with open(target_file, "w", encoding="utf-8") as f:
                        f.write(data)
                    print(f"[Skills] ✓ Downloaded '{slug}' from GitHub repository: {g_url}")
                    return True
        except Exception:
            continue

    # 3. If neither available, create local template
    target_file = os.path.join(target_dir, "SKILL.md")
    template = f"""---
name: {slug}
description: Skill for {slug} in Sayri / Pulsar OS
---

# Skill: {slug}

## Instructions
When the user asks for tasks related to {slug}, use appropriate bash commands.
"""
    with open(target_file, "w", encoding="utf-8") as f:
        f.write(template)
    print(f"[Skills] ✓ Created local skill template for '{slug}' at {target_file}")
    return True


def main() -> int:
    """CLI tool for sayri-skills."""
    args = sys.argv[1:]
    if not args or args[0] in ("-h", "--help"):
        print("Usage: sayri-skills <command> [args]")
        print("Commands:")
        print("  list                List installed skills")
        print("  read <name>         Read SKILL.md for a skill")
        print("  search <query>      Search ClawHub for skills")
        print("  install <name>      Install skill from ClawHub")
        return 0

    cmd = args[0]
    paths.ensure_dirs()

    if cmd == "list":
        skills = list_skills()
        if not skills:
            print(f"No skills installed in {paths.skills_dir()}")
            return 0
        print(f"Installed skills ({len(skills)}):")
        for s in skills:
            print(f"  - {s['name']}: {s['description']}")
        return 0

    elif cmd == "read":
        if len(args) < 2:
            print("Error: Specify skill name. Usage: sayri-skills read <name>")
            return 1
        content = read_skill(args[1])
        if content:
            print(content)
        else:
            print(f"Skill '{args[1]}' not found.")
            return 1
        return 0

    elif cmd == "search":
        query = " ".join(args[1:]) if len(args) > 1 else ""
        results = search_clawhub(query)
        print(f"ClawHub results for '{query}':")
        for r in results:
            print(f"  - {r.get('slug', r.get('name'))}: {r.get('description', '')}")
        return 0

    elif cmd == "install":
        if len(args) < 2:
            print("Error: Specify skill name. Usage: sayri-skills install <name>")
            return 1
        name = args[1]
        ok = install_skill(name)
        if ok:
            print(f"Skill '{name}' ready in {os.path.join(paths.skills_dir(), name)}")
        return 0 if ok else 1

    else:
        print(f"Unknown command: {cmd}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
