import { useCallback, useEffect, useRef, useState } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { FEATURE_SLIDES } from "../constants";

interface FeaturesPageProps {
  onContinue: () => void;
  onBack: () => void;
}

const SLIDE_MS = 10000;

export function FeaturesPage({ onContinue, onBack }: FeaturesPageProps) {
  const [index, setIndex] = useState(0);
  const videoRef = useRef<HTMLVideoElement>(null);
  const total = FEATURE_SLIDES.length;

  const next = useCallback(() => setIndex((i) => (i + 1) % total), [total]);
  const prev = useCallback(
    () => setIndex((i) => (i - 1 + total) % total),
    [total]
  );

  useEffect(() => {
    const t = setInterval(next, SLIDE_MS);
    return () => clearInterval(t);
  }, [next]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight") next();
      if (e.key === "ArrowLeft") prev();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [next, prev]);

  const slide = FEATURE_SLIDES[index];

  useEffect(() => {
    const v = videoRef.current;
    if (!v) return;
    v.load();
    let alive = true;
    const tryPlay = () => {
      if (alive) v.play().catch(() => {});
    };
    v.addEventListener("loadeddata", tryPlay);
    const t = window.setTimeout(tryPlay, 300);
    return () => {
      alive = false;
      window.clearTimeout(t);
      v.pause();
      v.removeEventListener("loadeddata", tryPlay);
    };
  }, [index]);

  const progressKey = `${slide.id}-${index}`;

  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div className="screen-enter glass flex h-[90vh] max-h-[740px] w-full max-w-[860px] flex-col overflow-hidden">
        {/* Slide counter + dots */}
        <div className="flex shrink-0 items-center justify-center gap-2 pt-6">
          {FEATURE_SLIDES.map((s, i) => (
            <button
              key={s.id}
              aria-label={`Slide ${i + 1}`}
              onClick={() => setIndex(i)}
              className={`slide-dot ${i === index ? "active" : ""}`}
            />
          ))}
        </div>

        {/* Progress bar */}
        <div className="mx-auto mt-3 h-0.5 w-40 overflow-hidden rounded-full bg-black/10">
          <div
            key={progressKey}
            className="progress-bar-fill h-full rounded-full bg-accent"
          />
        </div>

        {/* Content */}
        <main className="flex min-h-0 flex-1 flex-col items-center justify-center px-8 py-4 sm:px-12">
          <div
            key={slide.id}
            className="slide-fade flex w-full max-w-3xl flex-col items-center"
          >
            <h2 className="text-center text-[30px] font-semibold leading-tight text-text-primary sm:text-[36px]">
              {slide.title}
            </h2>
            <p className="mt-3 max-w-2xl text-center text-[15px] leading-relaxed text-text-secondary sm:text-[17px]">
              {slide.subtitle}
            </p>
            {slide.providers && (
              <div className="relative mt-8 w-full max-w-3xl">
                <div className="flex w-full flex-wrap items-center justify-center gap-3 sm:gap-4">
                  {slide.providers.map((p) => (
                    <div
                      key={p.name}
                      className="flex h-16 w-16 flex-col items-center justify-center rounded-2xl bg-white/80 p-2 shadow-md ring-1 ring-black/5"
                      title={p.name}
                    >
                      <img
                        src={p.src}
                        alt={p.name}
                        className="max-h-full max-w-full object-contain"
                        draggable={false}
                      />
                      <span className="mt-1 truncate text-[9px] font-medium text-text-secondary">
                        {p.name}
                      </span>
                    </div>
                  ))}
                </div>
                {/* Slide navigation overlays */}
                <button
                  aria-label="Previous slide"
                  onClick={prev}
                  className="absolute left-3 top-1/2 flex h-11 w-11 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full bg-black/30 text-white backdrop-blur transition-all hover:bg-black/50 active:scale-95"
                >
                  <ChevronLeft className="h-6 w-6" />
                </button>
                <button
                  aria-label="Next slide"
                  onClick={next}
                  className="absolute right-3 top-1/2 flex h-11 w-11 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full bg-black/30 text-white backdrop-blur transition-all hover:bg-black/50 active:scale-95"
                >
                  <ChevronRight className="h-6 w-6" />
                </button>
              </div>
            )}
            {slide.video && (
              <div className="relative mt-6 w-full max-w-3xl">
                <video
                  key={slide.video}
                  ref={videoRef}
                  src={slide.video}
                  muted
                  loop
                  playsInline
                  onError={(e) => {
                    const v = e.currentTarget;
                    v.removeAttribute("src");
                    v.load();
                  }}
                  className="w-full rounded-2xl border border-border shadow-lg"
                  style={{ maxHeight: "55vh" }}
                />
                {/* Slide navigation overlays */}
                <button
                  aria-label="Previous slide"
                  onClick={prev}
                  className="absolute left-3 top-1/2 flex h-11 w-11 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full bg-black/30 text-white backdrop-blur transition-all hover:bg-black/50 active:scale-95"
                >
                  <ChevronLeft className="h-6 w-6" />
                </button>
                <button
                  aria-label="Next slide"
                  onClick={next}
                  className="absolute right-3 top-1/2 flex h-11 w-11 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full bg-black/30 text-white backdrop-blur transition-all hover:bg-black/50 active:scale-95"
                >
                  <ChevronRight className="h-6 w-6" />
                </button>
              </div>
            )}
          </div>
        </main>

        {/* Footer */}
        <footer className="flex shrink-0 items-center justify-between border-t border-separator px-6 py-3.5 sm:px-8 sm:py-4">
          <button className="btn-secondary" onClick={onBack}>
            Back
          </button>
          <button className="btn-primary" onClick={onContinue}>
            Continue
          </button>
        </footer>
      </div>
    </div>
  );
}