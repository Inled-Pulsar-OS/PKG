import { ChevronLeft, ChevronRight, ExternalLink, QrCode } from "lucide-react";
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
        {slide.id === "flydrop" && <FlyDropSlice />}
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

function FlyDropSlice() {
  return (
    <div className="flex w-full min-h-[240px] flex-col sm:flex-row items-center justify-center gap-10 py-4">
      {/* Left: FlyDrop App Icon */}
      <div className="flex flex-col items-center text-center">
        <img
          src="./logos/flydrop.svg"
          alt="FlyDrop"
          className="h-28 w-28 object-contain drop-shadow-xl hover:scale-105 transition-transform"
          draggable={false}
        />
        <span className="mt-3 text-base font-semibold text-text-primary">FlyDrop</span>
        <span className="text-xs text-text-secondary">Our own AirDrop</span>
      </div>

      {/* Right: LocalSend QR code and centered link button underneath */}
      <div className="flex flex-col items-center">
        <div className="flex flex-col items-center bg-white p-3 rounded-2xl shadow-lg ring-1 ring-black/5">
          <img
            src="./logos/localsend-qr.svg"
            alt="Scan QR for LocalSend"
            className="h-28 w-28 object-contain"
            draggable={false}
          />
          <div className="mt-1 flex items-center gap-1 text-[11px] font-medium text-neutral-800">
            <QrCode className="h-3 w-3" />
            <span>Scan to get LocalSend</span>
          </div>
        </div>

        <a
          href="https://localsend.org"
          target="_blank"
          rel="noreferrer"
          className="mt-3 inline-flex items-center gap-2 rounded-xl bg-blue-600 px-5 py-2 text-xs font-semibold text-white shadow-md transition-all hover:bg-blue-500 active:scale-95"
        >
          <span>localsend.org</span>
          <ExternalLink className="h-3.5 w-3.5" />
        </a>
      </div>
    </div>
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
    <div className="flex w-full min-h-[220px] items-center justify-center py-6">
      <img
        src="./logos/pulsaros-optimizer.svg"
        alt="Pulsar OS Optimizer"
        className="h-32 w-32 object-contain drop-shadow-xl"
        draggable={false}
      />
    </div>
  );
}

function GlobalMenuSlice() {
  return (
    <div className="flex w-full min-h-[200px] flex-col items-center justify-center px-16 py-6">
      <div className="w-full max-w-[500px] rounded-xl bg-neutral-900/90 p-3 px-5 text-white shadow-2xl backdrop-blur-md ring-1 ring-white/10">
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
      <p className="mt-4 text-center text-[12px] text-text-secondary">
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
