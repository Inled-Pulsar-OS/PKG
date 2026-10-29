//! The single place that decides how to run a command as root.
//! Callers pass an argv list; nothing goes through a shell.

use crate::core::process::{argv, Runner};

/// What the host offers for privilege escalation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Env {
    pub is_root: bool,
    /// `sudo -n true` succeeds, so sudo will not prompt.
    pub sudo_noninteractive: bool,
    pub has_pkexec: bool,
    pub has_sudo: bool,
}

impl Env {
    pub fn detect(runner: &dyn Runner) -> Self {
        // SAFETY: geteuid has no preconditions and cannot fail.
        let is_root = unsafe { libc::geteuid() } == 0;
        let has_sudo = runner.which("sudo");
        Self {
            is_root,
            has_sudo,
            has_pkexec: runner.which("pkexec"),
            sudo_noninteractive: !is_root
                && has_sudo
                && runner
                    .capture(&argv(["sudo", "-n", "true"]))
                    .is_ok_and(|c| c.success),
        }
    }
}

/// Prefixes `cmd` with the right escalation tool: nothing when already root,
/// else `sudo -n`, `pkexec`, or interactive `sudo`, in that order.
pub fn wrap(cmd: &[String], env: &Env) -> Vec<String> {
    let prefix: &[&str] = if env.is_root {
        &[]
    } else if env.sudo_noninteractive {
        &["sudo", "-n"]
    } else if env.has_pkexec {
        &["pkexec"]
    } else if env.has_sudo {
        &["sudo"]
    } else {
        &[]
    };
    prefix
        .iter()
        .map(|s| s.to_string())
        .chain(cmd.iter().cloned())
        .collect()
}

/// Detects the environment and wraps `cmd`.
pub fn elevate(runner: &dyn Runner, cmd: &[String]) -> Vec<String> {
    wrap(cmd, &Env::detect(runner))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(root: bool, sudo_n: bool, pk: bool, sudo: bool) -> Env {
        Env {
            is_root: root,
            sudo_noninteractive: sudo_n,
            has_pkexec: pk,
            has_sudo: sudo,
        }
    }

    #[test]
    fn root_needs_no_prefix() {
        assert_eq!(
            wrap(&argv(["pacman", "-R"]), &env(true, false, true, true)),
            argv(["pacman", "-R"])
        );
    }

    #[test]
    fn prefers_noninteractive_sudo_then_pkexec_then_sudo() {
        let cmd = argv(["apt-get", "update"]);
        assert_eq!(
            wrap(&cmd, &env(false, true, true, true))[..2],
            argv(["sudo", "-n"])
        );
        assert_eq!(wrap(&cmd, &env(false, false, true, true))[0], "pkexec");
        assert_eq!(wrap(&cmd, &env(false, false, false, true))[0], "sudo");
        assert_eq!(wrap(&cmd, &env(false, false, false, false)), cmd);
    }

    #[test]
    fn arguments_stay_separate() {
        let cmd = argv(["pacman", "-S", "a b; touch /x"]);
        let out = wrap(&cmd, &env(false, false, true, false));
        assert_eq!(out.len(), 4);
        assert_eq!(out[3], "a b; touch /x");
    }
}
