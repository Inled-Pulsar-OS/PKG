import { SUPPORTED } from "@/modules/hello/contants";

export function getLang(): string {
  const forced = new URLSearchParams(window.location.search).get("lang");
  if (forced) {
    const norm = forced.split(".")[0].replace("-", "_");
    if (SUPPORTED.includes(norm)) return norm;
    const base = norm.slice(0, 2);
    if (SUPPORTED.includes(base)) return base;
  }
  const parts = navigator.language.split("-");
  const full = parts.slice(0, 2).join("_");
  if (SUPPORTED.includes(full)) return full;
  const code = parts[0].slice(0, 2);
  return SUPPORTED.includes(code) ? code : "en";
}


export async function loadAndAnimate(lang: string, container: HTMLDivElement | null) {
  if (!container) return;
  let svgEl: SVGSVGElement | null = null;
  for (const candidate of [lang, "en"]) {
    try {
      const res = await fetch(`/hello/svg/hello-${candidate}.svg`);
      console.log(`Fetching /hello/svg/hello-${candidate}.svg:`, res.status);
      const text = await res.text();
      const doc = new DOMParser().parseFromString(text, "image/svg+xml");
      svgEl = doc.querySelector("svg");
      if (svgEl) break;
    } catch {
      continue;
    }
  }
  if (!svgEl) return;

  const inner = svgEl.innerHTML;

  container.innerHTML = `
    <svg class="hello__svg" viewBox="${svgEl.getAttribute("viewBox") || "0 0 820 320"}">
      ${inner}
    </svg>
  `;

  const svg = container.querySelector("svg") as SVGSVGElement;

  const paths = svg.querySelectorAll("path");
  const totalDuration = 5000;
  let totalLength = 0;
  paths.forEach((p) => (totalLength += p.getTotalLength()));

  let currentTime = 0;
  paths.forEach((path) => {
    path.removeAttribute("stroke");
    path.removeAttribute("stroke-width");
    path.removeAttribute("stroke-linecap");
    path.removeAttribute("stroke-miterlimit");
    path.removeAttribute("fill");

    path.style.stroke = "black";
    path.style.fill = "none";
    path.style.strokeWidth = "32px";

    const len = path.getTotalLength();
    const duration = (len / totalLength) * totalDuration;

    path.style.strokeDasharray = String(len);
    path.style.strokeDashoffset = String(len);
    path.animate([{ strokeDashoffset: len }, { strokeDashoffset: 0 }], {
      duration,
      delay: currentTime,
      fill: "forwards",
      easing: "linear",
    });
    currentTime += duration;
  });
}
