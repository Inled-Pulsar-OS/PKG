import { useCallback, useEffect, useRef, useState } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import fallbackFeatures from "@/assets/site.json";
import type { FeatureItem } from "../types";

interface FeaturesPageProps {
  onBack: () => void;
  onFinish: () => void;
}

function resolveMediaUrl(url?: string): string {
  if (!url) return "";
  if (url.startsWith("http://") || url.startsWith("https://")) {
    return url;
  }
  return `https://bittenfruit.inled.es${url.startsWith("/") ? "" : "/"}${url}`;
}

function isVideoFile(url?: string): boolean {
  if (!url) return false;
  return /\.(mp4|webm|mov|m4v|ogg)(\?.*)?$/i.test(url);
}

interface MediaViewerProps {
  mediaUrl: string;
  posterUrl: string;
  isVideo: boolean;
  title: string;
}

function MediaViewer({ mediaUrl, posterUrl, isVideo, title }: MediaViewerProps) {
  const [progress, setProgress] = useState(15);
  const [showLoader, setShowLoader] = useState(true);
  const hasLoadedRef = useRef(false);
  const stallTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const videoRef = useRef<HTMLVideoElement>(null);

  // Stop video gracefully on unmount or slide change
  useEffect(() => {
    hasLoadedRef.current = false;
    setShowLoader(true);
    setProgress(15);

    if (stallTimerRef.current) {
      clearTimeout(stallTimerRef.current);
      stallTimerRef.current = null;
    }

    // Safety fallback: if media buffering takes > 3.5s, dismiss loader
    const fallbackTimeout = setTimeout(() => {
      setShowLoader(false);
      hasLoadedRef.current = true;
    }, 3500);

    const video = videoRef.current;
    return () => {
      clearTimeout(fallbackTimeout);
      if (stallTimerRef.current) {
        clearTimeout(stallTimerRef.current);
        stallTimerRef.current = null;
      }
      if (video) {
        try {
          video.pause();
        } catch {}
      }
    };
  }, [mediaUrl]);

  // Real progress calculation for video
  const handleVideoProgress = (e: React.SyntheticEvent<HTMLVideoElement>) => {
    const v = e.currentTarget;
    if (v.duration > 0 && v.buffered.length > 0) {
      const bufferedEnd = v.buffered.end(v.buffered.length - 1);
      const pct = Math.min(100, Math.round((bufferedEnd / v.duration) * 100));
      setProgress((prev) => Math.max(prev, pct));
      if (pct >= 50 || v.readyState >= 3) {
        if (!hasLoadedRef.current) {
          hasLoadedRef.current = true;
          setShowLoader(false);
        }
      }
    }
  };

  const handleCanPlay = () => {
    if (stallTimerRef.current) {
      clearTimeout(stallTimerRef.current);
      stallTimerRef.current = null;
    }
    setProgress(100);
    hasLoadedRef.current = true;
    setShowLoader(false);
  };

  const handlePlaying = () => {
    if (stallTimerRef.current) {
      clearTimeout(stallTimerRef.current);
      stallTimerRef.current = null;
    }
    hasLoadedRef.current = true;
    setShowLoader(false);
  };

  const handleTimeUpdate = () => {
    if (stallTimerRef.current) {
      clearTimeout(stallTimerRef.current);
      stallTimerRef.current = null;
    }
    if (showLoader) {
      setShowLoader(false);
      hasLoadedRef.current = true;
    }
  };

  const handleWaiting = () => {
    // Only show loader if already loaded once AND stall lasts > 5 seconds
    if (hasLoadedRef.current) {
      if (!stallTimerRef.current) {
        stallTimerRef.current = setTimeout(() => {
          setShowLoader(true);
        }, 5000);
      }
    } else {
      setShowLoader(true);
    }
  };

  return (
    <div className="relative flex h-full max-h-[420px] w-full max-w-[680px] items-center justify-center overflow-hidden rounded-2xl border border-black/10 bg-black/5 shadow-md">
      {isVideo ? (
        <video
          ref={videoRef}
          key={mediaUrl}
          src={mediaUrl}
          poster={posterUrl || undefined}
          autoPlay
          loop
          muted
          playsInline
          preload="metadata"
          onProgress={handleVideoProgress}
          onLoadedMetadata={handleVideoProgress}
          onLoadedData={handleCanPlay}
          onCanPlay={handleCanPlay}
          onPlaying={handlePlaying}
          onTimeUpdate={handleTimeUpdate}
          onWaiting={handleWaiting}
          onError={() => {
            setShowLoader(false);
            hasLoadedRef.current = true;
          }}
          className="h-full w-full object-contain"
        />
      ) : (
        <img
          key={mediaUrl}
          src={mediaUrl}
          alt={title}
          onLoad={() => {
            setProgress(100);
            hasLoadedRef.current = true;
            setShowLoader(false);
          }}
          onError={() => {
            setShowLoader(false);
            hasLoadedRef.current = true;
          }}
          draggable={false}
          className="h-full w-full object-contain"
        />
      )}

      {/* Progress Bar & Spinner Overlay */}
      {showLoader && (
        <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center bg-black/35 backdrop-blur-[2px] transition-opacity duration-200">
          <div className="flex flex-col items-center gap-3 rounded-2xl bg-black/75 px-7 py-5 shadow-2xl border border-white/10">
            {/* Minimalist Spinner */}
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-white/20 border-t-accent" />

            {/* Real Progress Bar */}
            <div className="flex flex-col items-center gap-1.5">
              <div className="h-1.5 w-44 overflow-hidden rounded-full bg-white/20">
                <div
                  className="h-full rounded-full bg-accent transition-all duration-150 ease-out"
                  style={{ width: `${Math.max(8, progress)}%` }}
                />
              </div>
              <span className="text-[11px] font-semibold text-white/80 tabular-nums">
                {progress}%
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export function FeaturesPage({ onBack, onFinish }: FeaturesPageProps) {
  const [features, setFeatures] = useState<FeatureItem[]>(fallbackFeatures as FeatureItem[]);
  const [index, setIndex] = useState(0);

  useEffect(() => {
    let isMounted = true;
    const fetchFeatures = async () => {
      try {
        const res = await fetch("https://bittenfruit.inled.es/site.json", { cache: "no-cache" });
        if (res.ok) {
          const data = await res.json();
          if (Array.isArray(data) && data.length > 0 && isMounted) {
            setFeatures(data);
            return;
          }
        }
      } catch {
        try {
          const localRes = await fetch("./site.json");
          if (localRes.ok) {
            const localData = await localRes.json();
            if (Array.isArray(localData) && localData.length > 0 && isMounted) {
              setFeatures(localData);
            }
          }
        } catch {
          // fallbackFeatures remains active
        }
      }
    };
    fetchFeatures();
    return () => {
      isMounted = false;
    };
  }, []);

  const total = features.length;
  const current = features[index] || features[0];

  const next = useCallback(() => {
    setIndex((i) => {
      if (i < total - 1) return i + 1;
      onFinish();
      return i;
    });
  }, [total, onFinish]);

  const prev = useCallback(() => {
    setIndex((i) => {
      if (i > 0) return i - 1;
      onBack();
      return i;
    });
  }, [onBack]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight") next();
      if (e.key === "ArrowLeft") prev();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [next, prev]);

  if (!current) {
    return null;
  }

  const mediaUrl = resolveMediaUrl(current.video);
  const posterUrl = resolveMediaUrl(current.poster);
  const isVideo = isVideoFile(mediaUrl);
  const isLast = index === total - 1;

  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-4 sm:p-8">
      <div className="screen-enter glass flex h-[90vh] max-h-[720px] w-full max-w-[880px] flex-col overflow-hidden">
        {/* Slide navigation dots */}
        <div className="flex shrink-0 items-center justify-center gap-1.5 pt-5 pb-1 px-4 overflow-x-auto">
          {features.map((f, i) => (
            <button
              key={`${f.title}-${i}`}
              type="button"
              aria-label={`Slide ${i + 1}`}
              onClick={() => setIndex(i)}
              className={`slide-dot cursor-pointer ${i === index ? "active" : ""}`}
            />
          ))}
        </div>

        {/* Header title & description */}
        <div className="shrink-0 px-8 pt-3 pb-2 text-center">
          <h2 className="text-[26px] sm:text-[32px] font-bold tracking-tight text-text-primary leading-tight">
            {current.title}
          </h2>
          <p className="mt-1.5 max-w-2xl mx-auto text-[14px] sm:text-[15px] leading-relaxed text-text-secondary">
            {current.description}
          </p>
        </div>

        {/* Media display area with chevrons */}
        <main className="relative flex min-h-0 flex-1 items-center justify-center px-4 sm:px-8 py-2">
          {/* Left Chevron */}
          <button
            type="button"
            aria-label="Previous slide"
            onClick={prev}
            className="absolute left-2 sm:left-4 top-1/2 -translate-y-1/2 z-30 flex h-11 w-11 cursor-pointer items-center justify-center rounded-full bg-black/50 text-white backdrop-blur shadow-lg transition-all hover:bg-black/75 hover:scale-105 active:scale-95"
          >
            <ChevronLeft className="h-6 w-6" />
          </button>

          <MediaViewer
            mediaUrl={mediaUrl}
            posterUrl={posterUrl}
            isVideo={isVideo}
            title={current.title}
          />

          {/* Right Chevron */}
          <button
            type="button"
            aria-label="Next slide"
            onClick={next}
            className="absolute right-2 sm:right-4 top-1/2 -translate-y-1/2 z-30 flex h-11 w-11 cursor-pointer items-center justify-center rounded-full bg-black/50 text-white backdrop-blur shadow-lg transition-all hover:bg-black/75 hover:scale-105 active:scale-95"
          >
            <ChevronRight className="h-6 w-6" />
          </button>
        </main>

        {/* Footer with action buttons */}
        <footer className="flex shrink-0 items-center justify-between border-t border-separator px-6 py-3.5 sm:px-8">
          <button type="button" className="btn-secondary" onClick={prev}>
            {index === 0 ? "Back" : "Previous"}
          </button>
          <span className="text-xs font-medium text-text-tertiary">
            {index + 1} / {total}
          </span>
          <button type="button" className="btn-primary" onClick={next}>
            {isLast ? "Get Started" : "Continue"}
          </button>
        </footer>
      </div>
    </div>
  );
}
