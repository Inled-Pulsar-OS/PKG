use std::process::Command;

#[derive(Clone, serde::Serialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
    pub active: bool,
}

pub fn get_resolutions() -> Vec<Resolution> {
    let output = Command::new("xrandr")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut resolutions = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        if !parts[0].contains('x') {
            continue;
        }
        let dims: Vec<&str> = parts[0].split('x').collect();
        if dims.len() != 2 {
            continue;
        }
        let w: u32 = match dims[0].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let h: u32 = match dims[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let active = parts[1..].iter().any(|s| *s == "*");
        if !resolutions.iter().any(|r: &Resolution| r.width == w && r.height == h) {
            resolutions.push(Resolution {
                width: w,
                height: h,
                active,
            });
        }
    }
    resolutions
}

fn get_active_output() -> Option<String> {
    let output = Command::new("xrandr")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let mut current_output = None;
    for line in output.lines() {
        if line.contains(" connected") {
            let name = line.split_whitespace().next()?.to_string();
            current_output = Some(name);
        }
        if line.contains(" connected") && line.contains(" active") {
            return Some(line.split_whitespace().next()?.to_string());
        }
    }
    current_output
}

pub fn set_resolution(width: u32, height: u32) -> Result<(), String> {
    let output = get_active_output().ok_or("No active display output found")?;
    let mode = format!("{}x{}", width, height);
    let out = Command::new("xrandr")
        .args(["--output", &output, "--mode", &mode])
        .output()
        .map_err(|e| format!("Failed to run xrandr: {}", e))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}
