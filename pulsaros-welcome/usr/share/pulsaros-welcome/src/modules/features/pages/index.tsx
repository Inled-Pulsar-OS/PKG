import React, { useCallback, useEffect, useRef, useState } from "react";
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
  children?: React.ReactNode;
}

function MediaViewer({ mediaUrl, posterUrl, isVideo, title, children }: MediaViewerProps) {
  const [progress, setProgress] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    setIsLoading(true);
    setProgress(5);

    if (!isVideo) {
      let active = true;
      const fetchImage = async () => {
        try {
          const response = await fetch(mediaUrl);
          if (!response.ok || !response.body) {
            if (active) {
              setProgress(100);
              setIsLoading(false);
            }
            return;
          }
          const contentLength = response.headers.get("content-length");
          const totalBytes = contentLength ? parseInt(contentLength, 10) : 0;
          let loadedBytes = 0;

          const reader = response.body.getReader();
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            loadedBytes += value ? value.length : 0;
            if (totalBytes > 0 && active) {
              const pct = Math.min(99, Math.round((loadedBytes / totalBytes) * 100));
              setProgress(pct);
            }
          }
          if (active) {
            setProgress(100);
            setIsLoading(false);
          }
        } catch {
          if (active) {
            setProgress(100);
            setIsLoading(false);
          }
        }
      };

      fetchImage();
      return () => {
        active = false;
      };
    }
  }, [mediaUrl, isVideo]);

  const updateVideoProgress = () => {
    const v = videoRef.current;
    if (v && v.duration > 0 && v.buffered.length > 0) {
      const bufferedEnd = v.buffered.end(v.buffered.length - 1);
      const pct = Math.min(100, Math.round((bufferedEnd / v.duration) * 100));
      setProgress(Math.max(10, pct));
      if (pct >= 90 || v.readyState >= 3) {
        setIsLoading(false);
      }
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
          onProgress={updateVideoProgress}
          onLoadedMetadata={updateVideoProgress}
          onLoadedData={updateVideoProgress}
          onCanPlay={() => {
            setProgress(100);
            setIsLoading(false);
          }}
          onPlaying={() => setIsLoading(false)}
          onWaiting={() => setIsLoading(true)}
          className="h-full w-full object-contain"
        />
      ) : (
        <img
          key={mediaUrl}
          src={mediaUrl}
          alt={title}
          onLoad={() => {
            setProgress(100);
            setIsLoading(false);
          }}
          draggable={false}
          className="h-full w-full object-contain"
        />
      )}

      {/* Real Progress Bar Overlay */}
      {isLoading && (
        <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/40 backdrop-blur-sm transition-opacity duration-200">
          <div className="flex flex-col items-center gap-2.5 rounded-2xl bg-black/65 px-6 py-4 shadow-2xl border border-white/10">
            <div className="flex items-center gap-2 text-white text-xs font-semibold tracking-wide">
              <span>Cargando {isVideo ? "vídeo" : "imagen"}</span>
              <span className="text-accent">{progress}%</span>
            </div>
            <div className="h-2 w-48 overflow-hidden rounded-full bg-white/20">
              <div
                className="h-full rounded-full bg-accent transition-all duration-150 ease-out"
                style={{ width: `${Math.max(5, progress)}%` }}
              />
            </div>
          </div>
        </div>
      )}

      {children}
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
    if (index < total - 1) {
      setIndex((i) => i + 1);
    } else {
      onFinish();
    }
  }, [index, total, onFinish]);

  const prev = useCallback(() => {
    if (index > 0) {
      setIndex((i) => i - 1);
    } else {
      onBack();
    }
  }, [index, onBack]);

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
        <main className="relative flex min-h-0 flex-1 items-center justify-center px-6 py-2">
          <MediaViewer
            mediaUrl={mediaUrl}
            posterUrl={posterUrl}
            isVideo={isVideo}
            title={current.title}
          >
            {/* Left Chevron */}
            <button
              aria-label="Previous slide"
              onClick={prev}
              className="absolute left-3 top-1/2 flex h-10 w-10 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full bg-black/40 text-white backdrop-blur transition-all hover:bg-black/60 active:scale-95"
            >
              <ChevronLeft className="h-6 w-6" />
            </button>

            {/* Right Chevron */}
            <button
              aria-label="Next slide"
              onClick={next}
              className="absolute right-3 top-1/2 flex h-10 w-10 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full bg-black/40 text-white backdrop-blur transition-all hover:bg-black/60 active:scale-95"
            >
              <ChevronRight className="h-6 w-6" />
            </button>
          </MediaViewer>
        </main>

        {/* Footer with action buttons */}
        <footer className="flex shrink-0 items-center justify-between border-t border-separator px-6 py-3.5 sm:px-8">
          <button className="btn-secondary" onClick={prev}>
            {index === 0 ? "Back" : "Previous"}
          </button>
          <span className="text-xs font-medium text-text-tertiary">
            {index + 1} / {total}
          </span>
          <button className="btn-primary" onClick={next}>
            {isLast ? "Get Started" : "Continue"}
          </button>
        </footer>
      </div>
    </div>
  );
}
