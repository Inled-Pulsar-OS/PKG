import { useEffect, useRef } from "react";
import { loadAndAnimate, getLang } from "@/modules/hello/utils";

interface HelloPageProps {
  onContinue: () => void;
}

export function HelloPage({ onContinue }: HelloPageProps) {
  const svgRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const prev = document.body.style.backgroundColor;
    document.body.style.backgroundColor = "transparent";
    const lang = getLang();
    loadAndAnimate(lang, svgRef.current);
    return () => { document.body.style.backgroundColor = prev; };
  }, []);

  return (
    <div className="flex h-screen w-screen flex-col items-center justify-center bg-transparent">
      <div
        ref={svgRef}
        className="flex flex-1 items-center justify-center px-[5%]"
        style={{ width: "90%", maxWidth: "900px", margin: "0 auto" }}
      />
      <button
        onClick={onContinue}
        className="cursor-pointer mb-20 rounded-full bg-white px-10 py-3 text-[15px] font-medium text-black shadow-md transition-all hover:bg-white/90 active:scale-95"
      >
        Continue
      </button>
    </div>
  );
}
