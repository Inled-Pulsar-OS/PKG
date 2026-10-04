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
      <p className="mt-2 max-w-2xl text-center text-[15px] leading-relaxed text-text-secondary sm:text-[17px]">
        {slide.subtitle}
      </p>

      <div className="relative mt-8 w-full max-w-3xl">
        {slide.id === "optimizer" && <OptimizerSlice />}
        {slide.id === "global-menu" && <GlobalMenuSlice />}
        {slide.id === "adblock" && <AdblockSlice />}
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

function OptimizerSlice() {
  return (
    <div className="glass-grouped flex w-full flex-col items-center justify-center rounded-2xl p-6 sm:p-8 shadow-inner">
      <div className="flex items-center gap-6">
        <img
          src="./logos/pulsaros-optimizer.svg"
          alt="Pulsar OS Optimizer"
          className="h-20 w-20 object-contain drop-shadow-md"
          draggable={false}
        />
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <span className="inline-block h-3 w-3 rounded-full bg-emerald-500 animate-pulse" />
            <span className="text-[14px] font-semibold text-text-primary">
              Habit Learning Engine Active
            </span>
          </div>
          <div className="h-2 w-56 overflow-hidden rounded-full bg-black/10">
            <div className="h-full w-2/3 rounded-full bg-gradient-to-r from-emerald-500 to-apple-blue" />
          </div>
          <div className="flex justify-between text-[11px] text-text-secondary">
            <span>Adaptive CPU & Memory Governor</span>
            <span className="font-semibold text-emerald-600">Zero-Lag</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function GlobalMenuSlice() {
  return (
    <div className="glass-grouped flex w-full flex-col items-center justify-center rounded-2xl p-6 shadow-inner">
      {/* Top Bar Preview */}
      <div className="w-full rounded-xl bg-neutral-900/90 p-2.5 px-4 text-white shadow-lg backdrop-blur-md">
        <div className="flex items-center justify-between text-[13px] font-medium">
          <div className="flex items-center gap-4">
            <img src="./logo.png" alt="Pulsar" className="h-4 w-4 object-contain brightness-200" />
            <span className="font-bold text-white">Application</span>
            <span className="text-white/80 hover:text-white cursor-default">File</span>
            <span className="text-white/80 hover:text-white cursor-default">Edit</span>
            <span className="text-white/80 hover:text-white cursor-default">View</span>
            <span className="text-white/80 hover:text-white cursor-default">Window</span>
            <span className="text-white/80 hover:text-white cursor-default">Help</span>
          </div>
          <div className="flex items-center gap-3 text-[12px] text-white/70">
            <span>100%</span>
            <span>Fri 14:20</span>
          </div>
        </div>
      </div>
      <p className="mt-3 text-center text-[12px] text-text-secondary">
        Native menus are automatically extracted from GTK, Qt and Electron apps directly to the top bar.
      </p>
    </div>
  );
}

function AdblockSlice() {
  return (
    <div className="flex w-full min-h-[220px] items-center justify-center py-6">
      <img
        src="./logos/pulsaros-hblock.svg"
        alt="Pulsar Adblock"
        className="h-32 w-32 object-contain drop-shadow-xl animate-pulse"
        draggable={false}
      />
    </div>
  );
}

