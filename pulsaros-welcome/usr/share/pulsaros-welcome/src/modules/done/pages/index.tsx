import { useEffect, useRef } from "react";
import { loadAndAnimate, getLang } from "@/modules/hello/utils";
import { writeSentinel, closeWindow } from "@/modules/core/api";

export function DonePage() {
  const svgRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const lang = getLang();
    loadAndAnimate(lang, svgRef.current);

    writeSentinel();

    const t = setTimeout(() => {
      closeWindow();
    }, 6000);

    return () => {
      clearTimeout(t);
    };
  }, []);

  return (
    <div className="screen-backdrop flex h-screen w-screen flex-col items-center justify-center">
      <div
        ref={svgRef}
        className="flex flex-1 items-center justify-center px-[5%]"
        style={{ width: "90%", maxWidth: "900px", margin: "0 auto" }}
      />
      <button
        onClick={() => {
          closeWindow();
        }}
        className="mb-20 cursor-pointer rounded-full bg-white px-10 py-3 text-[15px] font-medium text-black shadow-md transition-all hover:bg-white/90 active:scale-95 select-none"
      >
        Get Started
      </button>
    </div>
  );
}