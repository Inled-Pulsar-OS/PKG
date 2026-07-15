#!/usr/bin/env python3
import os
import sys
import subprocess
import time
import glob
import signal

if os.getuid() != 0:
    print("❌ This script must be run as root (sudo/pkexec).")
    sys.exit(1)

IMAGE_FILE = "/tmp/test-disk.img"
MOUNT_DIR = "/tmp/test-mnt"

def run_cmd(cmd):
    print(f"Running: {' '.join(cmd)}")
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Error executing command: {res.stderr.strip()}")
    return res

try:
    # 1. Clean up previous runs
    print("🧹 Cleaning up previous runs...")
    for bind_dir in ["bin", "lib", "lib64", "usr"]:
        mount_path = os.path.join(MOUNT_DIR, bind_dir)
        subprocess.run(["umount", "-f", mount_path], capture_output=True)

    subprocess.run(["umount", "-f", f"{MOUNT_DIR}/run"], capture_output=True)
    subprocess.run(["umount", "-R", f"{MOUNT_DIR}/sys"], capture_output=True)
    subprocess.run(["umount", "-f", f"{MOUNT_DIR}/proc"], capture_output=True)
    subprocess.run(["umount", "-R", f"{MOUNT_DIR}/dev"], capture_output=True)
    subprocess.run(["umount", "-f", MOUNT_DIR], capture_output=True)
    
    if os.path.exists(IMAGE_FILE):
        try:
            os.remove(IMAGE_FILE)
        except Exception:
            pass
    if os.path.exists(MOUNT_DIR):
        try:
            os.rmdir(MOUNT_DIR)
        except Exception:
            pass

    # 2. Create image and mount
    print("💾 Creating 100MB test disk image...")
    run_cmd(["dd", "if=/dev/zero", "of=" + IMAGE_FILE, "bs=1M", "count=100"])
    run_cmd(["mkfs.ext4", "-F", IMAGE_FILE])
    
    os.makedirs(MOUNT_DIR, exist_ok=True)
    run_cmd(["mount", IMAGE_FILE, MOUNT_DIR])

    # 3. Create virtual mount points
    for subdir in ["dev", "proc", "sys", "run", "etc"]:
        os.makedirs(os.path.join(MOUNT_DIR, subdir), exist_ok=True)

    # 4. Perform mounts exactly like recovery.py + host binaries for testing
    print("⚙️ Mounting virtual filesystems...")
    run_cmd(["mount", "--rbind", "/dev", os.path.join(MOUNT_DIR, "dev")])
    run_cmd(["mount", "--bind", "/proc", os.path.join(MOUNT_DIR, "proc")])
    run_cmd(["mount", "--rbind", "/sys", os.path.join(MOUNT_DIR, "sys")])
    run_cmd(["mount", "--bind", "/run", os.path.join(MOUNT_DIR, "run")])
    
    # Mount host binaries so we can run sleep in chroot
    for bind_dir in ["bin", "lib", "lib64", "usr"]:
        if os.path.exists("/" + bind_dir):
            os.makedirs(os.path.join(MOUNT_DIR, bind_dir), exist_ok=True)
            run_cmd(["mount", "--bind", "/" + bind_dir, os.path.join(MOUNT_DIR, bind_dir)])

    # 5. Start a background daemon process inside the chroot
    print("🎭 Starting a mock background process inside the chroot...")
    # Spawn a background sleep process using chroot
    chroot_proc = subprocess.Popen(["chroot", MOUNT_DIR, "sleep", "300"], 
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1) # Let it start
    print(f"Mock chroot process running with host PID: {chroot_proc.pid}")

    # Let's verify if our process killing logic detects it
    print("🔍 Testing chroot process detection...")
    my_pid = os.getpid()
    detected_pids = []
    for root_link in glob.glob("/proc/*/root"):
        try:
            real_root = os.readlink(root_link)
            # Use os.path.realpath to resolve any symlinks/namespaces correctly
            real_root_resolved = os.path.realpath(real_root)
            mount_dir_resolved = os.path.realpath(MOUNT_DIR)
            if real_root_resolved == mount_dir_resolved or real_root_resolved.startswith(mount_dir_resolved + "/"):
                pid_str = root_link.split("/")[2]
                pid = int(pid_str)
                if pid != my_pid:
                    detected_pids.append(pid)
        except Exception:
            pass
    print(f"Detected PIDs inside chroot: {detected_pids}")
    
    # 6. Run cleanup mounts
    print("🧹 Running cleanup_mounts...")
    
    # Process killing
    for pid in detected_pids:
        print(f"Killing PID {pid}...")
        try:
            os.kill(pid, signal.SIGKILL)
        except Exception as e:
            print(f"Failed to kill: {e}")
            
    # Also check if any host process is using the mount
    print("Checking if any host processes are holding files open in the mount...")
    lsof_res = subprocess.run(["lsof", MOUNT_DIR], capture_output=True, text=True)
    if lsof_res.stdout:
        print("⚠️ Host processes holding the mount:")
        print(lsof_res.stdout)
    else:
        print("No host processes holding the mount.")

    # Unmount host binaries first
    print("Unmounting host bind binaries...")
    for bind_dir in ["bin", "lib", "lib64", "usr"]:
        mount_path = os.path.join(MOUNT_DIR, bind_dir)
        if os.path.ismount(mount_path):
            subprocess.run(["umount", "-f", mount_path], capture_output=True)

    # Unmount sequence
    for mount_path, force_flag in [
        (os.path.join(MOUNT_DIR, "run"), "-f"),
        (os.path.join(MOUNT_DIR, "sys"), "-R"),
        (os.path.join(MOUNT_DIR, "proc"), "-f"),
        (os.path.join(MOUNT_DIR, "dev"), "-R"),
    ]:
        print(f"Unmounting {mount_path} with {force_flag}...")
        res = subprocess.run(["umount", force_flag, mount_path], capture_output=True, text=True)
        if res.returncode != 0:
            print(f"Failed, attempting lazy: {res.stderr.strip()}")
            subprocess.run(["umount", "-l", mount_path])
            
    print(f"Unmounting root mount {MOUNT_DIR}...")
    res = subprocess.run(["umount", "-f", MOUNT_DIR], capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Failed, attempting lazy: {res.stderr.strip()}")
        subprocess.run(["umount", "-l", MOUNT_DIR])

    # 7. Check if successfully unmounted
    print("📊 Verifying results...")
    mount_check = subprocess.run(["mount"], capture_output=True, text=True)
    if MOUNT_DIR in mount_check.stdout:
        print("❌ FAILED: The mount directory is STILL active!")
        fuser_res = subprocess.run(["fuser", "-v", MOUNT_DIR], capture_output=True, text=True)
        print("Fuser output:")
        print(fuser_res.stdout)
        print(fuser_res.stderr)
    else:
        print("✅ SUCCESS: The mount directory was successfully and cleanly unmounted!")

finally:
    # Cleanup files
    print("🧹 Cleaning up temp image file...")
    if os.path.exists(IMAGE_FILE):
        try:
            os.remove(IMAGE_FILE)
        except Exception:
            pass
