//! Raycast-compatible script commands: header parsing, discovery, running, and `Command` rows.
use super::commands::Command;
use serde::Deserialize;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command as Proc, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const OUTPUT_CAP: usize = 1024 * 1024;
const HEADER_LINES: usize = 60;
const MAX_DEPTH: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Silent,
    Compact,
    FullOutput,
    Inline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgType {
    #[default]
    Text,
    Password,
    Dropdown,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DropdownItem {
    pub title: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Argument {
    #[serde(rename = "type", default)]
    pub kind: ArgType,
    #[serde(default)]
    pub placeholder: String,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub data: Vec<DropdownItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub title: String,
    pub mode: Mode,
    pub icon: Option<String>,
    pub package_name: Option<String>,
    pub arguments: Vec<Argument>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    pub path: PathBuf,
    /// Path relative to the scripts dir it was found in.
    pub rel: String,
    pub header: Header,
}

/// Parse the first lines of a script. `Err(reason)` for anything invalid; never panics.
pub fn parse_header(text: &str) -> Result<Header, String> {
    let (mut schema, mut title, mut mode, mut icon, mut pkg) = (None, None, None, None, None);
    let mut args: Vec<(usize, Argument)> = Vec::new();
    for line in text.lines().take(HEADER_LINES) {
        let l = line.trim_start().trim_start_matches(['#', '/']).trim();
        let Some(rest) = l.strip_prefix("@raycast.") else { continue };
        let (key, val) = match rest.split_once(char::is_whitespace) {
            Some((k, v)) => (k, v.trim()),
            None => (rest, ""),
        };
        match key {
            "schemaVersion" => schema = Some(val.to_string()),
            "title" => title = Some(val.to_string()),
            "mode" => mode = Some(val.to_string()),
            "icon" => icon = Some(val.to_string()),
            "packageName" => pkg = Some(val.to_string()),
            k => {
                if let Some(n) = k.strip_prefix("argument").and_then(|n| n.parse::<usize>().ok()) {
                    let a: Argument = serde_json::from_str(val).map_err(|e| format!("bad argument{n} JSON: {e}"))?;
                    args.push((n, a));
                }
            }
        }
    }
    if schema.as_deref() != Some("1") {
        return Err("missing or unsupported @raycast.schemaVersion".into());
    }
    let title = title.filter(|t| !t.is_empty()).ok_or("missing @raycast.title")?;
    let mode = match mode.as_deref() {
        Some("silent") => Mode::Silent,
        Some("compact") => Mode::Compact,
        Some("fullOutput") => Mode::FullOutput,
        Some("inline") => Mode::Inline,
        Some(m) => return Err(format!("unknown @raycast.mode {m}")),
        None => return Err("missing @raycast.mode".into()),
    };
    args.sort_by_key(|(n, _)| *n);
    if args.len() > 3 {
        return Err("more than 3 arguments".into());
    }
    Ok(Header {
        title,
        mode,
        icon: icon.filter(|s| !s.is_empty()),
        package_name: pkg.filter(|s| !s.is_empty()),
        arguments: args.into_iter().map(|(_, a)| a).collect(),
    })
}

pub fn default_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config")).join("pulsaros-spotlight/scripts")
}

fn is_executable(p: &Path) -> bool {
    p.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

/// Discovered scripts plus a human-readable reason for every skipped file.
#[derive(Debug, Default)]
pub struct Discovery {
    pub scripts: Vec<Script>,
    pub skipped: Vec<String>,
}

fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Discovery) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
            continue;
        }
        if p.is_dir() {
            if depth < MAX_DEPTH {
                walk(root, &p, depth + 1, out);
            }
            continue;
        }
        let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().into_owned();
        if !is_executable(&p) {
            out.skipped.push(format!("{rel}: not executable"));
            continue;
        }
        let Ok(bytes) = std::fs::read(&p) else {
            out.skipped.push(format!("{rel}: unreadable"));
            continue;
        };
        match parse_header(&String::from_utf8_lossy(&bytes)) {
            Ok(header) => out.scripts.push(Script { path: p, rel, header }),
            Err(e) => out.skipped.push(format!("{rel}: {e}")),
        }
    }
}

/// Scan `dirs` (default dir first, then extras). Missing dirs are ignored.
pub fn discover(dirs: &[PathBuf]) -> Discovery {
    let mut out = Discovery::default();
    for d in dirs {
        walk(d, d, 0, &mut out);
    }
    out
}

pub fn to_command(s: &Script) -> Command {
    Command {
        id: format!("script:{}", s.rel),
        title: s.header.title.clone(),
        breadcrumb: s.header.package_name.clone().unwrap_or_else(|| "Script Commands".into()),
        keywords: vec!["script".into()],
        icon: s.header.icon.clone().unwrap_or_else(|| "utilities-terminal".into()),
        argv: vec![s.path.to_string_lossy().into_owned()],
        confirm: false,
        state: None,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RawOutput {
    pub stdout: String,
    pub stderr: String,
    pub code: Option<i32>,
    pub timed_out: bool,
}

/// Executes argv with a timeout and per-stream output cap. Injectable for tests.
pub trait Runner {
    fn run(&self, argv: &[String], timeout: Duration, cap: usize) -> std::io::Result<RawOutput>;
}

pub struct SystemRunner;

fn drain<R: Read + Send + 'static>(mut r: R, cap: usize) -> Arc<Mutex<Vec<u8>>> {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let b = buf.clone();
    std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        while let Ok(n) = r.read(&mut chunk) {
            if n == 0 {
                break;
            }
            let mut g = b.lock().unwrap();
            let room = cap.saturating_sub(g.len());
            g.extend_from_slice(&chunk[..n.min(room)]); // keep draining so the child never blocks
        }
    });
    buf
}

impl Runner for SystemRunner {
    fn run(&self, argv: &[String], timeout: Duration, cap: usize) -> std::io::Result<RawOutput> {
        let (cmd, args) = argv.split_first().ok_or_else(|| std::io::Error::other("empty argv"))?;
        let mut child = Proc::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()?;
        let out = drain(child.stdout.take().unwrap(), cap);
        let err = drain(child.stderr.take().unwrap(), cap);
        let start = Instant::now();
        let (code, timed_out) = loop {
            if let Some(st) = child.try_wait()? {
                break (st.code(), false);
            }
            if start.elapsed() >= timeout {
                // SAFETY: signalling the child's own process group (pgid == pid via process_group(0)).
                unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
                let _ = child.kill();
                let _ = child.wait();
                break (None, true);
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        std::thread::sleep(Duration::from_millis(20)); // let readers flush the last bytes
        let s = |b: &Arc<Mutex<Vec<u8>>>| String::from_utf8_lossy(&b.lock().unwrap()).into_owned();
        Ok(RawOutput { stdout: s(&out), stderr: s(&err), code, timed_out })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Nothing to show.
    Silent,
    /// Last output line, for a toast.
    Compact(String),
    /// Whole output for a pushed view.
    FullOutput(String),
    /// Last output line, for the row subtitle.
    Inline(String),
    /// Spawn error, non-zero exit or timeout; message for a failure toast.
    Failed(String),
}

fn last_line(s: &str) -> String {
    s.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string()
}

/// Map raw process output to a per-mode result.
pub fn interpret(mode: Mode, raw: &RawOutput) -> Outcome {
    if raw.timed_out {
        return Outcome::Failed("timed out".into());
    }
    if raw.code != Some(0) {
        let msg = [last_line(&raw.stderr), last_line(&raw.stdout)].into_iter().find(|m| !m.is_empty());
        return Outcome::Failed(msg.unwrap_or_else(|| format!("exit code {}", raw.code.map_or("?".into(), |c| c.to_string()))));
    }
    match mode {
        Mode::Silent => Outcome::Silent,
        Mode::Compact => Outcome::Compact(last_line(&raw.stdout)),
        Mode::Inline => Outcome::Inline(last_line(&raw.stdout)),
        Mode::FullOutput => Outcome::FullOutput(raw.stdout.clone()),
    }
}

/// Run `script` with `args` (argv, never a shell). Missing required arguments fail without running.
pub fn run_with(script: &Script, args: &[String], runner: &dyn Runner, timeout: Duration) -> Outcome {
    let declared = &script.header.arguments;
    let missing = declared.iter().enumerate().any(|(i, a)| !a.optional && args.get(i).is_none_or(|v| v.is_empty()));
    if missing {
        return Outcome::Failed("missing required argument".into());
    }
    let mut argv = vec![script.path.to_string_lossy().into_owned()];
    argv.extend(args.iter().take(declared.len()).cloned());
    match runner.run(&argv, timeout, OUTPUT_CAP) {
        Ok(raw) => interpret(script.header.mode, &raw),
        Err(e) => Outcome::Failed(e.to_string()),
    }
}

pub fn run(script: &Script, args: &[String], runner: &dyn Runner) -> Outcome {
    run_with(script, args, runner, DEFAULT_TIMEOUT)
}

#[cfg(test)]
#[path = "../tests/script_commands_tests.rs"]
mod tests;
