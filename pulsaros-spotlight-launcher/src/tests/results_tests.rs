use super::*;

#[test]
fn thumb_generates_scaled_from_real_image() {
    let url = "file:///usr/share/backgrounds/pulsar-os-tahoe.png";
    let pb = generate_thumb_pixbuf(url, 48);
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
