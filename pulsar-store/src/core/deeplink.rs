//! Reading "install this item" requests from the command line.
//!
//! Accepted forms: `pulsar://install/<id>` and `--install <id>`.

/// The item id requested by `args`, if any. Ids must be a single plain path segment.
pub fn install_request<S: AsRef<str>>(args: &[S]) -> Option<String> {
    let mut it = args.iter().map(AsRef::as_ref);
    while let Some(arg) = it.next() {
        let id = if let Some(rest) = arg.strip_prefix("pulsar://install/") {
            Some(rest.trim().trim_end_matches('/').to_string())
        } else if arg == "--install" {
            it.next().map(|s| s.trim().to_string())
        } else {
            arg.strip_prefix("--install=").map(|s| s.trim().to_string())
        };
        if let Some(id) = id {
            return valid_id(&id).then_some(id);
        }
    }
    None
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && !id.contains(['/', '\\', '?', '#', '\0'])
        && !id.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_forms() {
        assert_eq!(
            install_request(&["store", "pulsar://install/org.x.App"]).as_deref(),
            Some("org.x.App")
        );
        assert_eq!(
            install_request(&["store", "--install", "a@b"]).as_deref(),
            Some("a@b")
        );
        assert_eq!(
            install_request(&["store", "--install=thing"]).as_deref(),
            Some("thing")
        );
        assert_eq!(
            install_request(&["store", "pulsar://install/x/"]).as_deref(),
            Some("x")
        );
    }

    #[test]
    fn rejects_bad_requests() {
        assert_eq!(install_request(&["store"]), None);
        assert_eq!(install_request(&["store", "--install"]), None);
        assert_eq!(install_request(&["pulsar://install/"]), None);
        assert_eq!(install_request(&["pulsar://install/a/b"]), None);
        assert_eq!(install_request(&["pulsar://install/a?x=1"]), None);
        assert_eq!(install_request(&["pulsar://open/x"]), None);
    }
}
