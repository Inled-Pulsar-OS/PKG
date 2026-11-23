use gtk4::glib;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Duration;

pub(super) static THUMBNAIL_CACHE: OnceLock<Mutex<HashMap<String, Option<gtk4::gdk::Texture>>>> = OnceLock::new();

pub(super) struct ThumbPixels {
    pub(super) bytes: glib::Bytes,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) rowstride: i32,
    pub(super) has_alpha: bool,
    pub(super) bits_per_sample: i32,
}

pub(super) struct ThumbJob {
    pub(super) url: String,
    pub(super) cache_key: String,
    pub(super) size: i32,
    pub(super) disk_png: std::path::PathBuf,
}

pub(super) fn fnv1a64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// Creates the worker pool and returns (job sender, result receiver).
pub(super) fn spawn_thumbnail_pool() -> (
    mpsc::Sender<ThumbJob>,
    mpsc::Receiver<(String, Option<ThumbPixels>)>,
) {
    const WORKERS: usize = 4;
    let (job_tx, job_rx) = mpsc::channel::<ThumbJob>();
    let (result_tx, result_rx) = mpsc::channel::<(String, Option<ThumbPixels>)>();

    let worker_txs: Vec<mpsc::Sender<ThumbJob>> = (0..WORKERS)
        .map(|_| {
            let (wtx, wrx) = mpsc::channel::<ThumbJob>();
            let rt = result_tx.clone();
            std::thread::spawn(move || {
                while let Ok(job) = wrx.recv() {
                    let pixels = generate_thumb_pixels(&job);
                    let _ = rt.send((job.cache_key, pixels));
                }
            });
            wtx
        })
        .collect();

    std::thread::spawn(move || {
        let mut i = 0usize;
        while let Ok(job) = job_rx.recv() {
            let _ = worker_txs[i % WORKERS].send(job);
            i += 1;
        }
    });

    (job_tx, result_rx)
}

pub(super) fn is_image_file(url: &str) -> bool {
    let path = url.trim_start_matches("file://");
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "avif" | "tiff" | "ico")
}

pub(super) fn is_pdf_file(url: &str) -> bool {
    url.trim_start_matches("file://").ends_with(".pdf")
}

pub(super) fn is_video_file(url: &str) -> bool {
    let ext = Path::new(url.trim_start_matches("file://"))
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "wmv" | "m4v" | "3gp")
}

pub(super) fn has_thumbnail_support(url: &str) -> bool {
    is_image_file(url) || is_pdf_file(url) || is_video_file(url)
}

/// How long a thumbnail helper (`pdftoppm`, `ffmpeg`) may run before it is killed.
pub(super) const THUMB_HELPER_TIMEOUT: Duration = Duration::from_secs(10);

/// Picks the directory that holds scratch roots: `$XDG_RUNTIME_DIR` when it is an existing
/// absolute directory, else the per-user cache dir. Never falls back to `/tmp`; with neither
/// available it returns `None` and the caller skips the thumbnail.
pub(super) fn scratch_base(
    runtime_dir: Option<std::path::PathBuf>,
    cache_home: Option<std::path::PathBuf>,
    cache_dir: Option<std::path::PathBuf>,
    home: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    runtime_dir
        .filter(|p| p.is_absolute() && p.is_dir())
        .or_else(|| {
            cache_home
                .filter(|p| p.is_absolute())
                .or(cache_dir)
                .or_else(|| home.map(|h| h.join(".cache")))
                .map(|c| c.join("pulsaros-spotlight"))
        })
}

/// A private, exclusively created scratch directory for one thumbnail job. It lives under
/// `$XDG_RUNTIME_DIR` (or the cache dir), never in world-writable `/tmp`, so another local
/// user can't pre-create the output path as a symlink. Removed on drop.
pub(super) struct ScratchDir(std::path::PathBuf);

impl ScratchDir {
    pub(super) fn new() -> Option<Self> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        let base = scratch_base(
            std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from),
            std::env::var_os("XDG_CACHE_HOME").map(std::path::PathBuf::from),
            dirs::cache_dir(),
            dirs::home_dir(),
        )?;
        let root = base.join("pulsaros-spotlight-scratch");
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&root).ok()?;
        let meta = std::fs::symlink_metadata(&root).ok()?;
        // SAFETY: getuid has no preconditions.
        let me = unsafe { libc::getuid() };
        if !meta.is_dir() || meta.uid() != me || meta.mode() & 0o077 != 0 {
            return None;
        }
        // mkdir fails if the name exists, so the job directory is ours alone.
        let dir = root.join(format!("job-{}-{}", std::process::id(), out_counter()));
        std::fs::DirBuilder::new().mode(0o700).create(&dir).ok()?;
        Some(ScratchDir(dir))
    }

    pub(super) fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs `cmd` and kills it if it outlives `limit`. Returns true when it exited by itself with
/// success; a spawn error, timeout or non-zero exit is false.
pub(super) fn run_with_timeout(mut cmd: Command, limit: Duration) -> bool {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    let Ok(mut child) = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
    else {
        return false;
    };
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => return st.success(),
            Ok(None) if start.elapsed() >= limit => {
                // SAFETY: signals the helper's own process group (pgid == pid via process_group(0)).
                unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return false,
        }
    }
}

// Runs on a worker thread: decodes + scales into a thread-safe Pixbuf.
pub(super) fn generate_thumb_pixbuf(url: &str, size: i32) -> Option<gdk_pixbuf::Pixbuf> {
    let path = url.trim_start_matches("file://");

    if is_pdf_file(url) {
        let scratch = ScratchDir::new()?;
        let base = scratch.path().join("page");
        let mut cmd = Command::new("pdftoppm");
        cmd.args(["-png", "-singlefile", "-r", "72", "-l", "1", path]).arg(&base);
        if !run_with_timeout(cmd, THUMB_HELPER_TIMEOUT) {
            return None;
        }
        gdk_pixbuf::Pixbuf::from_file_at_scale(base.with_extension("png"), size, size, true).ok()
    } else if is_video_file(url) {
        let scratch = ScratchDir::new()?;
        let out = scratch.path().join("frame.jpg");
        let mut cmd = Command::new("ffmpeg");
        // -n: never overwrite; -nostdin: never read the terminal.
        cmd.args(["-nostdin", "-n", "-ss", "00:00:01", "-i", path, "-frames:v", "1", "-q:v", "5"]).arg(&out);
        if !run_with_timeout(cmd, THUMB_HELPER_TIMEOUT) {
            return None;
        }
        gdk_pixbuf::Pixbuf::from_file_at_scale(&out, size, size, true).ok()
    } else if is_image_file(url) {
        gdk_pixbuf::Pixbuf::from_file_at_scale(path, size, size, true).ok()
    } else {
        None
    }
}

pub(super) fn generate_thumb_pixels(job: &ThumbJob) -> Option<ThumbPixels> {
    let pb = if job.disk_png.exists() {
        gdk_pixbuf::Pixbuf::from_file_at_scale(&job.disk_png, job.size, job.size, true).ok()
    } else {
        let pb = generate_thumb_pixbuf(&job.url, job.size);
        if let Some(p) = &pb {
            if let Some(parent) = job.disk_png.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = p.savev(&job.disk_png, "png", &[]);
        }
        pb
    };
    pb.map(|p| ThumbPixels {
        bytes: p.read_pixel_bytes(),
        width: p.width(),
        height: p.height(),
        rowstride: p.rowstride(),
        has_alpha: p.has_alpha(),
        bits_per_sample: p.bits_per_sample(),
    })
}

pub(super) fn thumb_cache_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::cache_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    base.join("pulsaros-spotlight").join("thumbnails")
}

pub(super) fn out_counter() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

