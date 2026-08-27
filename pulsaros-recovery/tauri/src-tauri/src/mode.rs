use std::path::Path;

/// Detect which system state the app is running in.
///
/// Priority:
/// 1. `/etc/pulsar-need-setup` exists → first-boot OOTB
/// 2. `/run/live/medium` or `/cdrom` exists → live USB installer
/// 3. Default → recovery environment
pub fn detect_mode() -> &'static str {
    if Path::new("/etc/pulsar-need-setup").exists() {
        return "ootb";
    }
    if Path::new("/run/live/medium").exists() || Path::new("/cdrom").exists() {
        return "installer";
    }
    "recovery"
}
