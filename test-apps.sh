#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# ==============================================================================
# Pulsar OS - Local GUI Application Test Launcher (Safe Simulation Sandbox)
# ==============================================================================

import os
import sys
import subprocess

RECOVERY_PATH = "/home/jaime/Documentos/pulsar/PKG/pulsaros-recovery/usr/share/pulsaros-recovery/recovery.py"
WELCOME_PATH = "/home/jaime/Documentos/pulsar/PKG/pulsaros-welcome/usr/share/pulsaros/welcome_ootb.py"

def check_dependencies():
    try:
        import gi
        gi.require_version('Gtk', '4.0')
        gi.require_version('Adw', '1')
        print("✅ Python GTK4 and Libadwaita dependencies found.")
    except Exception as e:
        print("❌ Error: Python GTK4 and Libadwaita (python3-gi / libadwaita-1) are missing.")
        print("To install them on Debian/Ubuntu/Pulsar:")
        print("   sudo apt update && sudo apt install python3-gi gir1.2-gtk-4.0 gir1.2-adw-1")
        sys.exit(1)

def main():
    print("==================================================")
    print("  Pulsar OS - Local Safe Testing Launcher")
    print("==================================================")
    check_dependencies()
    
    print("\nSelect application to launch in simulation mode:")
    print("1) Recovery Assistant & Native Installer (recovery.py)")
    print("2) OOTB Setup Assistant (welcome_ootb.py)")
    print("3) Exit")
    
    choice = input("\nOption (1-3): ").strip()
    
    env = os.environ.copy()
    env["TEST_MODE"] = "1"
    
    if choice == "1":
        print("\n🚀 Launching Recovery Assistant in simulated sandbox...")
        subprocess.run(["python3", RECOVERY_PATH], env=env)
    elif choice == choice == "2":
        print("\n🚀 Launching OOTB Setup Assistant in simulated sandbox...")
        subprocess.run(["python3", WELCOME_PATH], env=env)
    else:
        print("\nExiting.")

if __name__ == "__main__":
    main()
