use super::*;

#[test]
fn thumb_generates_scaled_from_real_image() {
    let path = "/usr/share/backgrounds/pulsar-os-tahoe.png";
    if !std::path::Path::new(path).exists() {
        // Only a Pulsar OS install ships this wallpaper; skip on build hosts and containers.
        eprintln!("skipped: {path} not installed");
        return;
    }
    let pb = generate_thumb_pixbuf(&format!("file://{path}"), 48);
    assert!(pb.is_some(), "el pixbuf de la imagen debería generarse");
    let p = pb.unwrap();
    assert!(p.width() > 0 && p.height() > 0);
    assert!(p.width() <= 48 && p.height() <= 48, "debe caber en 48px: {}x{}", p.width(), p.height());
}

#[test]
fn thumb_generates_from_real_pdf() {
    let url = "file:///usr/share/doc/glm/manual.pdf";
    if std::path::Path::new(url.trim_start_matches("file://")).exists() {
        let pb = generate_thumb_pixbuf(url, 48);
        assert!(pb.is_some(), "el pixbuf del PDF debería generarse");
    }
}

#[test]
fn thumb_rejects_unsupported_files() {
    let pb = generate_thumb_pixbuf("file:///etc/hostname", 48);
    assert!(pb.is_none());
}

#[test]
fn thumb_detects_extensions() {
    assert!(is_image_file("file:///tmp/a.PNG"));
    assert!(is_pdf_file("file:///tmp/a.pdf"));
    assert!(is_video_file("file:///tmp/a.Mp4"));
    assert!(!is_image_file("file:///tmp/a.txt"));
}

#[test]
fn helper_past_timeout_is_killed() {
    let start = std::time::Instant::now();
    let mut cmd = Command::new("sleep");
    cmd.arg("30");
    assert!(!run_with_timeout(cmd, Duration::from_millis(200)));
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
fn fast_helper_succeeds() {
    let cmd = Command::new("true");
    assert!(run_with_timeout(cmd, Duration::from_secs(5)));
}

#[test]
fn scratch_dir_is_private_and_cleaned_up() {
    use std::os::unix::fs::PermissionsExt;
    let kept;
    {
        let s = ScratchDir::new().expect("scratch dir");
        kept = s.path().to_path_buf();
        assert_eq!(std::fs::metadata(&kept).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(kept.parent().and_then(|p| p.file_name()).unwrap(), "pulsaros-spotlight-scratch");
    }
    assert!(!kept.exists());
}

#[test]
fn scratch_base_prefers_runtime_dir_then_cache_and_never_tmp() {
    use std::path::PathBuf;
    let rt = std::env::temp_dir(); // an existing absolute directory
    assert_eq!(scratch_base(Some(rt.clone()), None, None, None), Some(rt));

    // runtime dir unset or not a directory: per-user cache location
    let cache = Some(PathBuf::from("/home/u/.cache"));
    let expect = Some(PathBuf::from("/home/u/.cache/pulsaros-spotlight"));
    assert_eq!(scratch_base(None, None, cache.clone(), None), expect);
    assert_eq!(scratch_base(Some(PathBuf::from("/no/such/dir")), None, cache, None), expect);

    // relative XDG_CACHE_HOME is ignored per the spec
    assert_eq!(
        scratch_base(None, Some(PathBuf::from("rel/cache")), None, Some(PathBuf::from("/home/u"))),
        Some(PathBuf::from("/home/u/.cache/pulsaros-spotlight"))
    );

    // nothing resolvable: no scratch space, not /tmp
    assert_eq!(scratch_base(None, None, None, None), None);
}
