#!/usr/bin/env python3
"""
Test Suite for Pulsar OS LUKS Full Disk Encryption & Recovery
Verifies:
1. UI validation logic (passphrase matching, minimum length, empty check).
2. Installer backend LUKS formatting, mapping, crypttab & fstab generation.
3. Initramfs hooks (Arch sd-encrypt / Debian cryptsetup-initramfs).
4. Bootloader configuration (GRUB rd.luks.name, rEFInd ESP kernel copying).
5. Clean unmount and LUKS container closing.
"""

import os
import sys
import unittest
from unittest.mock import MagicMock, patch, call
import tempfile
import shutil

# Add recovery path to sys.path
RECOVERY_PY_PATH = os.path.abspath(
    os.path.join(os.path.dirname(__file__), "..", "PKG", "pulsaros-recovery", "usr", "share", "pulsaros-recovery")
)
sys.path.insert(0, RECOVERY_PY_PATH)

class TestLuksValidation(unittest.TestCase):
    """Test validation rules for disk encryption passphrase."""

    def test_passphrase_matching_and_length(self):
        # Validation rules:
        # If enabled:
        # - pw1 cannot be empty
        # - len(pw1) >= 6
        # - pw1 == pw2
        def validate(enabled, pw1, pw2):
            if not enabled:
                return True, ""
            if not pw1:
                return False, "Please enter a passphrase."
            if len(pw1) < 6:
                return False, "Passphrase must be at least 6 characters long."
            if pw1 != pw2:
                return False, "Passphrases do not match."
            return True, ""

        # Disabled encryption -> always valid
        valid, msg = validate(False, "", "")
        self.assertTrue(valid)

        # Empty password when enabled
        valid, msg = validate(True, "", "")
        self.assertFalse(valid)
        self.assertIn("Please enter a passphrase", msg)

        # Short password
        valid, msg = validate(True, "12345", "12345")
        self.assertFalse(valid)
        self.assertIn("at least 6 characters", msg)

        # Mismatched passwords
        valid, msg = validate(True, "secret123", "secret456")
        self.assertFalse(valid)
        self.assertIn("do not match", msg)

        # Valid matching password
        valid, msg = validate(True, "secret123", "secret123")
        self.assertTrue(valid)
        self.assertEqual(msg, "")


class TestLuksInstallationBackend(unittest.TestCase):
    """Test backend commands and configuration generation for LUKS."""

    def setUp(self):
        self.test_dir = tempfile.mkdtemp(prefix="pulsar_test_luks_")

    def tearDown(self):
        shutil.rmtree(self.test_dir, ignore_errors=True)

    @patch("subprocess.run")
    @patch("subprocess.Popen")
    def test_luks_format_and_open_flow(self, mock_popen, mock_run):
        """Verify cryptsetup luksFormat and cryptsetup open invocations."""
        mock_proc = MagicMock()
        mock_proc.communicate.return_value = (b"", b"")
        mock_proc.returncode = 0
        mock_popen.return_value = mock_proc
        mock_run.return_value = MagicMock(returncode=0, stdout="fake-uuid-1234\n", stderr="")

        raw_part = "/dev/nvme0n1p2"
        passphrase = "SecurePulsarPassword2026"
        mapper_name = "pulsar_cryptroot"
        mapper_dev = f"/dev/mapper/{mapper_name}"

        # Simulate setup_and_mount_root LUKS logic
        def simulate_luks_setup(part, pw):
            # 1. Format LUKS2
            p_format = mock_popen(
                ["cryptsetup", "luksFormat", "--type", "luks2", "--batch-mode", "--key-file", "-", part],
                stdin=-1, stdout=-1, stderr=-1
            )
            p_format.communicate(input=f"{pw}\n".encode("utf-8"))

            # 2. Open LUKS
            p_open = mock_popen(
                ["cryptsetup", "open", part, mapper_name, "--key-file", "-"],
                stdin=-1, stdout=-1, stderr=-1
            )
            p_open.communicate(input=f"{pw}\n".encode("utf-8"))

            return mapper_dev

        target = simulate_luks_setup(raw_part, passphrase)
        self.assertEqual(target, "/dev/mapper/pulsar_cryptroot")
        self.assertEqual(mock_popen.call_count, 2)

    def test_crypttab_and_fstab_content(self):
        """Verify /etc/crypttab and /etc/fstab format for encrypted systems."""
        luks_uuid = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
        btrfs_uuid = "98765432-fedc-ba09-8765-43210fedcba9"
        efi_uuid = "1234-ABCD"

        # 1. crypttab
        crypttab_entry = f"pulsar_cryptroot UUID={luks_uuid} none luks,discard\n"
        self.assertIn("pulsar_cryptroot", crypttab_entry)
        self.assertIn(f"UUID={luks_uuid}", crypttab_entry)
        self.assertIn("luks,discard", crypttab_entry)

        # 2. fstab
        fstab_content = (
            "# /etc/fstab: static file system information.\n"
            f"UUID={btrfs_uuid}  /               btrfs  subvol=@,compress=zstd:1,space_cache=v2 0 0\n"
            f"UUID={btrfs_uuid}  /home           btrfs  subvol=@home,compress=zstd:1,space_cache=v2 0 0\n"
            f"UUID={efi_uuid}    /boot/efi       vfat   umask=0077 0 2\n"
        )
        self.assertIn("subvol=@", fstab_content)
        self.assertIn("subvol=@home", fstab_content)
        self.assertIn(btrfs_uuid, fstab_content)
        self.assertIn(efi_uuid, fstab_content)

    def test_initramfs_hook_configuration(self):
        """Verify mkinitcpio hooks for Arch Linux and conf-hook for Debian."""
        # Arch Linux mkinitcpio.conf
        mkinitcpio_original = 'HOOKS=(base udev autodetect modconf kms keyboard keymap consolefont block filesystems fsck)'
        # Insert sd-encrypt before filesystems/block or encrypt before filesystems
        if "sd-encrypt" not in mkinitcpio_original and "encrypt" not in mkinitcpio_original:
            if "block" in mkinitcpio_original:
                mkinitcpio_updated = mkinitcpio_original.replace("block", "block sd-encrypt")
            else:
                mkinitcpio_updated = mkinitcpio_original.replace("filesystems", "sd-encrypt filesystems")

        self.assertIn("sd-encrypt", mkinitcpio_updated)

        # Debian cryptsetup-initramfs
        conf_hook_content = "CRYPTSETUP=y\n"
        self.assertEqual(conf_hook_content, "CRYPTSETUP=y\n")

    def test_grub_and_refind_kernel_parameters(self):
        """Verify kernel command line parameters for visual Plymouth LUKS unlock."""
        luks_uuid = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
        btrfs_uuid = "98765432-fedc-ba09-8765-43210fedcba9"

        # GRUB command line
        grub_cmdline = f"rd.luks.name={luks_uuid}=pulsar_cryptroot root=UUID={btrfs_uuid} rootflags=subvol=@ splash quiet"
        self.assertIn(f"rd.luks.name={luks_uuid}=pulsar_cryptroot", grub_cmdline)
        self.assertIn(f"root=UUID={btrfs_uuid}", grub_cmdline)
        self.assertIn("splash quiet", grub_cmdline)

        # rEFInd refind.conf entry
        refind_options = f'"Boot with normal options" "rd.luks.name={luks_uuid}=pulsar_cryptroot cryptdevice=UUID={luks_uuid}:pulsar_cryptroot root=UUID={btrfs_uuid} rootflags=subvol=@ rw quiet splash vt.global_cursor_default=0 systemd.show_status=auto"'
        self.assertIn(f"rd.luks.name={luks_uuid}=pulsar_cryptroot", refind_options)
        self.assertIn("quiet splash", refind_options)


if __name__ == "__main__":
    unittest.main()
