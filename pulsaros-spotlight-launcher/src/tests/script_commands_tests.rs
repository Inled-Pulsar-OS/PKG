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
