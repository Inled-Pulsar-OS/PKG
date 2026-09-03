import { ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "@/modules/ui/utils";
import { FeatureSlide } from "../types";

interface SliceCardProps {
  slide: FeatureSlide;
  idx: number;
  index: number;
  prev: () => void;
  next: () => void;
}

export function SliceCard({ slide, idx, index, prev, next }: SliceCardProps) {
  return (
    <div
      className={cn(
        "slide-fade flex w-full max-w-3xl flex-col items-center",
        idx !== index && "hidden",
      )}
    >
      <h2 className="text-center text-[30px] font-semibold leading-tight text-text-primary sm:text-[36px]">
        {slide.title}
      </h2>
      <p className="mt-3 max-w-2xl text-center text-[15px] leading-relaxed text-text-secondary sm:text-[17px]">
        {slide.subtitle}
      </p>

      <div className="relative mt-8 w-full max-w-3xl">
        {slide.providers && <PictureSlice slide={slide} />}
        {slide.video && <AnimatedSlice src={slide.video} />}
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
    </div>
  );
}

/** Animated WebP — no video pipeline, no GStreamer, no DMA-BUF. Just an <img>. */
function AnimatedSlice({ src }: { src: string }) {
  return (
    <img
      src={src}
      alt=""
      draggable={false}
      className="w-full rounded-2xl border border-border shadow-lg"
      style={{ maxHeight: "55vh", objectFit: "contain" }}
    />
  );
}

function PictureSlice({ slide }: { slide: FeatureSlide }) {
  return (
    <div className="flex w-full flex-wrap items-center justify-center gap-3 sm:gap-4">
      {slide?.providers?.map((p) => (
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
  );
}
