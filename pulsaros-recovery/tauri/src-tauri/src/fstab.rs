use std::fs;

pub fn generate_fstab(
    root_path: &str,
    btrfs_uuid: &str,
    efi_uuid: &str,
) -> Result<(), String> {
    let mut content = format!(
        "# /etc/fstab: Pulsar OS Btrfs Configuration\n\
         UUID={} /               btrfs   subvol=@,compress=zstd:1,space_cache=v2 0 0\n\
         UUID={} /home           btrfs   subvol=@home,compress=zstd:1,space_cache=v2 0 0\n",
        btrfs_uuid, btrfs_uuid,
    );

    if !efi_uuid.is_empty() {
        content.push_str(&format!(
            "UUID={} /boot/efi       vfat    umask=0077 0 2\n",
            efi_uuid
        ));
    }

    fs::write(format!("{}/etc/fstab", root_path), content)
        .map_err(|e| format!("Failed to write fstab: {}", e))
}
