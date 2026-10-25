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
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;

    fn hdr(mode: &str) -> String {
        format!("#!/bin/bash\n# @raycast.schemaVersion 1\n# @raycast.title T\n# @raycast.mode {mode}\n")
    }

    #[test]
    fn parses_every_mode() {
        for (m, e) in [("silent", Mode::Silent), ("compact", Mode::Compact), ("fullOutput", Mode::FullOutput), ("inline", Mode::Inline)] {
            assert_eq!(parse_header(&hdr(m)).unwrap().mode, e);
        }
    }

    #[test]
    fn parses_full_header_slash_style() {
        let t = "#!/usr/bin/env node\n// @raycast.schemaVersion 1\n// @raycast.title Hello World\n// @raycast.mode compact\n\
// @raycast.icon 🤖\n// @raycast.packageName Demo\n\
// @raycast.argument2 { \"type\": \"dropdown\", \"placeholder\": \"d\", \"data\": [{\"title\":\"A\",\"value\":\"a\"}] }\n\
// @raycast.argument1 { \"type\": \"text\", \"placeholder\": \"name\", \"optional\": true }\n";
        let h = parse_header(t).unwrap();
        assert_eq!(h.title, "Hello World");
        assert_eq!(h.icon.as_deref(), Some("🤖"));
        assert_eq!(h.package_name.as_deref(), Some("Demo"));
        assert_eq!(h.arguments.len(), 2);
        assert!(h.arguments[0].optional);
        assert_eq!(h.arguments[1].kind, ArgType::Dropdown);
        assert_eq!(h.arguments[1].data[0].value, "a");
    }

    #[test]
    fn password_type() {
        let t = format!("{}# @raycast.argument1 {{\"type\":\"password\",\"placeholder\":\"p\"}}\n", hdr("silent"));
        assert_eq!(parse_header(&t).unwrap().arguments[0].kind, ArgType::Password);
    }

    #[test]
    fn bad_headers_are_errors_not_panics() {
        for t in [
            "",
            "\0\0\0",
            "# @raycast.",
            "# @raycast.title",
            "#!/bin/sh\necho hi",
            "# @raycast.schemaVersion 2\n# @raycast.title T\n# @raycast.mode silent",
            "# @raycast.schemaVersion 1\n# @raycast.mode silent",
            "# @raycast.schemaVersion 1\n# @raycast.title T",
            "# @raycast.schemaVersion 1\n# @raycast.title T\n# @raycast.mode bogus",
            "# @raycast.schemaVersion 1\n# @raycast.title T\n# @raycast.mode silent\n# @raycast.argument1 {not json",
            "# @raycast.schemaVersion 1\n# @raycast.title T\n# @raycast.mode silent\n# @raycast.argument1 {\"type\":\"nope\"}",
            "# @raycast.schemaVersion 1\n# @raycast.title \n# @raycast.mode silent",
            "# @raycast.argumentX {}\n# @raycast.argument99999999999999999999 {}",
        ] {
            assert!(parse_header(t).is_err(), "{t:?}");
        }
    }

    #[test]
    fn unknown_keys_ignored_and_too_many_args() {
        assert!(parse_header(&format!("{}# @raycast.currentDirectoryPath ~\n", hdr("silent"))).is_ok());
        let many: String = (1..=4).map(|i| format!("# @raycast.argument{i} {{\"type\":\"text\"}}\n")).collect();
        assert!(parse_header(&format!("{}{many}", hdr("silent"))).is_err());
    }

    struct Fake {
        raw: RawOutput,
        seen: RefCell<Vec<Vec<String>>>,
    }
    impl Runner for Fake {
        fn run(&self, argv: &[String], _: Duration, _: usize) -> std::io::Result<RawOutput> {
            self.seen.borrow_mut().push(argv.to_vec());
            Ok(self.raw.clone())
        }
    }
    fn fake(stdout: &str, code: i32) -> Fake {
        Fake { raw: RawOutput { stdout: stdout.into(), code: Some(code), ..Default::default() }, seen: RefCell::new(vec![]) }
    }
    fn script(mode: &str, extra: &str) -> Script {
        Script { path: "/s/a.sh".into(), rel: "a.sh".into(), header: parse_header(&format!("{}{extra}", hdr(mode))).unwrap() }
    }

    #[test]
    fn run_each_mode() {
        let f = fake("one\ntwo\n\n", 0);
        assert_eq!(run(&script("silent", ""), &[], &f), Outcome::Silent);
        assert_eq!(run(&script("compact", ""), &[], &f), Outcome::Compact("two".into()));
        assert_eq!(run(&script("inline", ""), &[], &f), Outcome::Inline("two".into()));
        assert_eq!(run(&script("fullOutput", ""), &[], &f), Outcome::FullOutput("one\ntwo\n\n".into()));
    }

    #[test]
    fn failure_modes() {
        let s = script("silent", "");
        assert_eq!(run(&s, &[], &fake("", 3)), Outcome::Failed("exit code 3".into()));
        let mut f = fake("out", 1);
        f.raw.stderr = "boom\n".into();
        assert_eq!(run(&s, &[], &f), Outcome::Failed("boom".into()));
        f.raw.timed_out = true;
        assert_eq!(run(&s, &[], &f), Outcome::Failed("timed out".into()));
    }

    #[test]
    fn args_are_argv_and_required_checked() {
        let s = script("compact", "# @raycast.argument1 {\"type\":\"text\"}\n# @raycast.argument2 {\"type\":\"text\",\"optional\":true}\n");
        let f = fake("ok", 0);
        assert!(matches!(run(&s, &[], &f), Outcome::Failed(_)));
        assert!(matches!(run(&s, &["".into()], &f), Outcome::Failed(_)));
        assert!(f.seen.borrow().is_empty());
        run(&s, &["a b; echo pwned".into(), "x".into(), "extra".into()], &f);
        assert_eq!(f.seen.borrow()[0], vec!["/s/a.sh", "a b; echo pwned", "x"]);
    }

    #[test]
    fn to_command_row() {
        let s = script("silent", "# @raycast.packageName Pkg\n# @raycast.icon star\n");
        let c = to_command(&Script { rel: "sub/a.sh".into(), ..s.clone() });
        assert_eq!((c.id.as_str(), c.breadcrumb.as_str(), c.icon.as_str()), ("script:sub/a.sh", "Pkg", "star"));
        assert_eq!(c.argv, vec!["/s/a.sh"]);
        let d = to_command(&script("silent", ""));
        assert_eq!(d.breadcrumb, "Script Commands");
    }

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pulsaros-script-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }
    fn write(p: &Path, body: &str, mode: u32) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[test]
    fn discovery_filters_and_skips() {
        let d = tmpdir("disc");
        let extra = tmpdir("disc-extra");
        write(&d.join("ok.sh"), &hdr("silent"), 0o755);
        write(&d.join("pkg/nested.sh"), &hdr("compact"), 0o755);
        write(&d.join("noexec.sh"), &hdr("silent"), 0o644);
        write(&d.join("bad.sh"), "#!/bin/sh\n", 0o755);
        write(&d.join(".hidden.sh"), &hdr("silent"), 0o755);
        write(&extra.join("e.sh"), &hdr("inline"), 0o755);
        let r = discover(&[d.clone(), extra, PathBuf::from("/nonexistent-dir-xyz")]);
        let mut rels: Vec<_> = r.scripts.iter().map(|s| s.rel.clone()).collect();
        rels.sort();
        assert_eq!(rels, ["e.sh", "ok.sh", "pkg/nested.sh"]);
        assert_eq!(r.skipped.len(), 2);
        assert!(r.skipped.iter().any(|s| s.contains("noexec.sh") && s.contains("not executable")));
        assert!(r.skipped.iter().any(|s| s.contains("bad.sh")));
    }

    #[test]
    fn default_dir_suffix() {
        assert!(default_dir().ends_with("pulsaros-spotlight/scripts"));
    }

    fn real(mode: &str, body: &str, extra: &str, tag: &str) -> Script {
        let d = tmpdir(tag);
        write(&d.join("s.sh"), &format!("{}{extra}{body}\n", hdr(mode)), 0o755);
        discover(&[d]).scripts.remove(0)
    }

    #[test]
    fn real_run_every_mode() {
        let body = "echo first; echo last";
        assert_eq!(run(&real("silent", body, "", "r1"), &[], &SystemRunner), Outcome::Silent);
        assert_eq!(run(&real("compact", body, "", "r2"), &[], &SystemRunner), Outcome::Compact("last".into()));
        assert_eq!(run(&real("inline", body, "", "r3"), &[], &SystemRunner), Outcome::Inline("last".into()));
        assert_eq!(run(&real("fullOutput", body, "", "r4"), &[], &SystemRunner), Outcome::FullOutput("first\nlast\n".into()));
    }

    #[test]
    fn real_run_args_and_failure() {
        let s = real("compact", "echo \"[$1]\"", "# @raycast.argument1 {\"type\":\"text\"}\n", "r5");
        assert_eq!(run(&s, &["a b $HOME".into()], &SystemRunner), Outcome::Compact("[a b $HOME]".into()));
        let f = real("silent", "echo nope >&2; exit 2", "", "r6");
        assert_eq!(run(&f, &[], &SystemRunner), Outcome::Failed("nope".into()));
    }

    #[test]
    fn real_timeout_kills() {
        let s = real("silent", "sleep 10", "", "r7");
        let t = Instant::now();
        assert_eq!(run_with(&s, &[], &SystemRunner, Duration::from_millis(200)), Outcome::Failed("timed out".into()));
        assert!(t.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn real_output_cap() {
        let argv = ["sh".to_string(), "-c".into(), "head -c 3000000 /dev/zero | tr '\\0' a".into()];
        let r = SystemRunner.run(&argv, Duration::from_secs(10), OUTPUT_CAP).unwrap();
        assert_eq!(r.stdout.len(), OUTPUT_CAP);
        assert_eq!(r.code, Some(0));
    }

    #[test]
    fn spawn_failure_is_failed() {
        let s = Script { path: "/nonexistent/x".into(), ..script("silent", "") };
        assert!(matches!(run(&s, &[], &SystemRunner), Outcome::Failed(_)));
    }
}
