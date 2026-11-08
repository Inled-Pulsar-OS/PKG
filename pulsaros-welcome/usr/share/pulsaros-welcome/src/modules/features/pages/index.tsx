import { useCallback, useEffect, useRef, useState } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import fallbackFeatures from "@/assets/site.json";
import fallbackContributors from "@/assets/contributors.json";
import type { FeatureItem, Contributor } from "../types";

interface FeaturesPageProps {
  onBack: () => void;
  onFinish: () => void;
}

async function openExternalUrl(url: string) {
  if (!url) return;
  try {
    const { open } = await import("@tauri-apps/plugin-shell");
    await open(url);
  } catch {
    window.open(url, "_blank");
  }
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

interface ContributorsViewProps {
  contributors: Contributor[];
}

function ContributorsView({ contributors }: ContributorsViewProps) {
  return (
    <div className="relative flex h-full max-h-[420px] w-full max-w-[720px] flex-col items-center justify-center overflow-hidden rounded-2xl border border-black/10 bg-black/5 shadow-md backdrop-blur p-4 sm:p-6">
      <div className="flex h-full w-full flex-wrap items-center justify-center gap-4 sm:gap-6 overflow-y-auto py-2">
        {contributors.map((person) => (
          <div
            key={person.name}
            className="flex flex-col items-center gap-2.5 rounded-2xl bg-white/75 dark:bg-black/50 p-4 border border-black/5 dark:border-white/10 shadow-sm transition-all hover:scale-105 hover:shadow-md w-[140px] sm:w-[155px]"
          >
            <div className="relative h-16 w-16 sm:h-20 sm:w-20 overflow-hidden rounded-full border-2 border-accent/40 shadow-sm bg-neutral-200">
              <img
                src={person.avatar}
                alt={person.name}
                className="h-full w-full object-cover"
                loading="lazy"
                onError={(e) => {
                  (e.currentTarget as HTMLImageElement).src = "/logo.png";
                }}
              />
            </div>
            <span
              className="text-center text-[13px] sm:text-[14px] font-semibold tracking-tight text-text-primary leading-snug line-clamp-2 min-h-[2.5em] flex items-center justify-center"
              title={person.name}
            >
              {person.name}
            </span>
            <div className="flex items-center gap-2 pt-0.5">
              {person.discord && (
                <button
                  type="button"
                  onClick={() => openExternalUrl(person.discord)}
                  className="flex h-7 w-7 items-center justify-center rounded-full bg-[#5865F2] text-white shadow-sm hover:opacity-90 active:scale-95 transition-all cursor-pointer"
                  title={`Discord: @${person.discordUser || person.name}`}
                  aria-label="Discord"
                >
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
                    <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994.021-.041.001-.09-.041-.106a13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.929 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.894.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.028zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
                  </svg>
                </button>
              )}
              {person.github && (
                <button
                  type="button"
                  onClick={() => openExternalUrl(person.github)}
                  className="flex h-7 w-7 items-center justify-center rounded-full bg-black/80 dark:bg-white/15 text-white shadow-sm hover:opacity-90 active:scale-95 transition-all cursor-pointer"
                  title={`GitHub: ${person.name}`}
                  aria-label="GitHub"
                >
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
                    <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
                  </svg>
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export function FeaturesPage({ onBack, onFinish }: FeaturesPageProps) {
  const [features, setFeatures] = useState<FeatureItem[]>(fallbackFeatures as FeatureItem[]);
  const [contributors, setContributors] = useState<Contributor[]>(fallbackContributors as Contributor[]);
  const [index, setIndex] = useState(0);

  useEffect(() => {
    let isMounted = true;
    const fetchSiteData = async () => {
      // 1. Fetch features
      try {
        const res = await fetch("https://bittenfruit.inled.es/site.json", { cache: "no-cache" });
        if (res.ok) {
          const data = await res.json();
          if (Array.isArray(data) && data.length > 0 && isMounted) {
            setFeatures(data);
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
        } catch {}
      }

      // 2. Fetch contributors
      try {
        const contribRes = await fetch("https://bittenfruit.inled.es/contributors.json", { cache: "no-cache" });
        if (contribRes.ok) {
          const contribData = await contribRes.json();
          if (Array.isArray(contribData) && contribData.length > 0 && isMounted) {
            setContributors(contribData);
          }
        }
      } catch {
        try {
          const localContribRes = await fetch("./contributors.json");
          if (localContribRes.ok) {
            const localContribData = await localContribRes.json();
            if (Array.isArray(localContribData) && localContribData.length > 0 && isMounted) {
              setContributors(localContribData);
            }
          }
        } catch {}
      }
    };

    fetchSiteData();
    return () => {
      isMounted = false;
    };
  }, []);

  const total = features.length + 1; // features + contributors slide
  const isContributorsSlide = index === features.length;
  const current = features[index] || features[0];
  const isLast = index === total - 1;

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

  const mediaUrl = isContributorsSlide ? "" : resolveMediaUrl(current.video);
  const posterUrl = isContributorsSlide ? "" : resolveMediaUrl(current.poster);
  const isVideo = isContributorsSlide ? false : isVideoFile(mediaUrl);

  const displayTitle = isContributorsSlide
    ? "The people behind Pulsar OS"
    : current.title;

  const displayDescription = isContributorsSlide
    ? "When you contribute to Pulsar OS, no matter how, you get recognized. Join our community!"
    : current.description;

  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-4 sm:p-8">
      <div className="screen-enter glass flex h-[90vh] max-h-[720px] w-full max-w-[880px] flex-col overflow-hidden">
        {/* Slide navigation dots */}
        <div className="flex shrink-0 items-center justify-center gap-1.5 pt-5 pb-1 px-4 overflow-x-auto">
          {Array.from({ length: total }).map((_, i) => (
            <button
              key={`dot-${i}`}
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
            {displayTitle}
          </h2>
          <p className="mt-1.5 max-w-2xl mx-auto text-[14px] sm:text-[15px] leading-relaxed text-text-secondary">
            {displayDescription}
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

          {isContributorsSlide ? (
            <ContributorsView contributors={contributors} />
          ) : (
            <MediaViewer
              mediaUrl={mediaUrl}
              posterUrl={posterUrl}
              isVideo={isVideo}
              title={current.title}
            />
          )}

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
