import { useCallback, useEffect, useState } from "react";
import { FEATURE_SLIDES, SLIDE_MS } from "../constants";
import { SliceCard } from "../components/slice-card";

interface FeaturesPageProps {
  onContinue: () => void;
  onBack: () => void;
}

export function FeaturesPage({ onContinue, onBack }: FeaturesPageProps) {
  const [index, setIndex] = useState(0);
  const total = FEATURE_SLIDES.length;

  const next = useCallback(() => setIndex((i) => (i + 1) % total), [total]);
  const prev = useCallback(
    () => setIndex((i) => (i - 1 + total) % total),
    [total],
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

  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center p-5 sm:p-8">
      <div className="screen-enter glass flex h-[90vh] max-h-185 w-full max-w-215 flex-col overflow-hidden">
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
          <div className="progress-bar-fill h-full rounded-full bg-accent" />
        </div>

        {/* Content */}
        <main className="flex min-h-0 flex-1 flex-col items-center justify-center px-8 py-4 sm:px-12">
          {FEATURE_SLIDES.map((slide, idx) => (
            <SliceCard
              key={slide.id}
              idx={idx}
              index={index}
              slide={slide}
              prev={prev}
              next={next}
            />
          ))}
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
