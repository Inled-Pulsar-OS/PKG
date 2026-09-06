#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Pulsar Store Backends Package."""

from .flatpak_backend import FlatpakBackend
from .system_backend import SystemPackageBackend, SystemPackageBackend as SystemBackend
from .gnome_ext_backend import GnomeExtensionBackend
from .sayri_skill_backend import SayriSkillBackend
from .sayri_plugin_backend import SayriPluginBackend

__all__ = [
    "FlatpakBackend",
    "SystemPackageBackend",
    "SystemBackend",
    "GnomeExtensionBackend",
    "SayriSkillBackend",
    "SayriPluginBackend",
]
