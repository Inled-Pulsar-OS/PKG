//! Running external commands. Arguments are always an argv list, never a shell string.

use crate::core::error::{Result, StoreError};
use crate::core::ports::CancelToken;
use std::collections::VecDeque;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const TAIL_LINES: usize = 15;

#[derive(Debug, Clone)]
pub struct RunOutput {
    pub success: bool,
    /// Last lines of combined output, for error messages.
    pub tail: Vec<String>,
}

impl RunOutput {
    pub fn tail_text(&self) -> String {
        self.tail.join("\n")
    }
}

#[derive(Debug, Clone)]
pub struct Captured {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait Runner: Send + Sync {
    /// Runs a command, passing each output line (stdout and stderr) to `on_line`.
    /// Kills the process and returns `Cancelled` when `cancel` is set.
    fn run(
        &self,
        argv: &[String],
        on_line: &mut dyn FnMut(&str),
        cancel: &CancelToken,
    ) -> Result<RunOutput>;

    /// Runs a command to completion with `LC_ALL=C` and returns its output.
    fn capture(&self, argv: &[String]) -> Result<Captured>;

    /// True when `name` is an executable on `PATH`.
    fn which(&self, name: &str) -> bool;
}

pub struct SystemRunner;

fn spawn_error(argv: &[String], e: std::io::Error) -> StoreError {
    StoreError::CommandFailed(format!("{}: {e}", argv.first().map_or("", String::as_str)))
}

/// Sends each line of `reader` to `tx`. Splits on `\n` and `\r`, because
/// progress bars redraw with carriage returns.
fn pump<R: Read + Send + 'static>(mut reader: R, tx: mpsc::Sender<String>) {
    std::thread::spawn(move || {
        let mut line = Vec::new();
        let mut buf = [0u8; 4096];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            for &b in &buf[..n] {
                if b == b'\n' || b == b'\r' {
                    if !line.is_empty() {
                        let _ = tx.send(String::from_utf8_lossy(&line).into_owned());
                        line.clear();
                    }
                } else {
                    line.push(b);
                }
            }
        }
        if !line.is_empty() {
            let _ = tx.send(String::from_utf8_lossy(&line).into_owned());
        }
    });
}

impl Runner for SystemRunner {
    fn run(
        &self,
        argv: &[String],
        on_line: &mut dyn FnMut(&str),
        cancel: &CancelToken,
    ) -> Result<RunOutput> {
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| StoreError::CommandFailed("empty command".into()))?;
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| spawn_error(argv, e))?;

        let (tx, rx) = mpsc::channel();
        if let Some(out) = child.stdout.take() {
            pump(out, tx.clone());
        }
        if let Some(err) = child.stderr.take() {
            pump(err, tx.clone());
        }
        drop(tx);

        let mut tail: VecDeque<String> = VecDeque::new();
        loop {
            if cancel.is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(StoreError::Cancelled);
            }
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(line) => {
                    on_line(&line);
                    if tail.len() == TAIL_LINES {
                        tail.pop_front();
                    }
                    tail.push_back(line);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        let status = child.wait()?;
        Ok(RunOutput {
            success: status.success(),
            tail: tail.into(),
        })
    }

    fn capture(&self, argv: &[String]) -> Result<Captured> {
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| StoreError::CommandFailed("empty command".into()))?;
        let out = Command::new(program)
            .args(args)
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .output()
            .map_err(|e| spawn_error(argv, e))?;
        Ok(Captured {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    fn which(&self, name: &str) -> bool {
        std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let p = dir.join(name);
                p.is_file() && is_executable(&p)
            })
        })
    }
}

fn is_executable(p: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

/// Builds an argv list from string literals.
pub fn argv<const N: usize>(parts: [&str; N]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

/// Reads a progress percentage such as `Installing... 45%` from a line of output.
pub fn parse_percent(line: &str) -> Option<f32> {
    let idx = line.rfind('%')?;
    let digits: String = line[..idx]
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let n: f32 = digits.parse().ok()?;
    (n <= 100.0).then_some(n / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_parsing() {
        assert_eq!(parse_percent("Installing… 45%"), Some(0.45));
        assert_eq!(parse_percent("100%"), Some(1.0));
        assert_eq!(parse_percent("no number"), None);
        assert_eq!(parse_percent("200%"), None);
    }

    #[test]
    fn runs_and_streams_output() {
        let mut lines = Vec::new();
        let out = SystemRunner
            .run(
                &argv(["sh", "-c", "echo one; echo two >&2"]),
                &mut |l| lines.push(l.to_string()),
                &CancelToken::new(),
            )
            .unwrap();
        assert!(out.success);
        lines.sort();
        assert_eq!(lines, ["one", "two"]);
    }

    #[test]
    fn failure_keeps_tail() {
        let out = SystemRunner
            .run(
                &argv(["sh", "-c", "echo boom; exit 3"]),
                &mut |_| {},
                &CancelToken::new(),
            )
            .unwrap();
        assert!(!out.success);
        assert_eq!(out.tail_text(), "boom");
    }

    #[test]
    fn cancel_kills_the_process() {
        let cancel = CancelToken::new();
        let c2 = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            c2.cancel();
        });
        let start = std::time::Instant::now();
        let r = SystemRunner.run(&argv(["sleep", "10"]), &mut |_| {}, &cancel);
        assert!(matches!(r, Err(StoreError::Cancelled)));
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn capture_and_which() {
        let c = SystemRunner
            .capture(&argv(["sh", "-c", "echo $LC_ALL"]))
            .unwrap();
        assert_eq!(c.stdout.trim(), "C");
        assert!(SystemRunner.which("sh"));
        assert!(!SystemRunner.which("definitely-not-a-command-xyz"));
        assert!(SystemRunner
            .capture(&argv(["definitely-not-a-command-xyz"]))
            .is_err());
    }
}
