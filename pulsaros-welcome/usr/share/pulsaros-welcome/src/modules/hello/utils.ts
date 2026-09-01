import { SUPPORTED } from "@/modules/hello/contants";
import { HELLO_SVGS } from "@/modules/hello/svg-map";

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

function extractSvg(xml: string): { viewBox: string; body: string } | null {
  const doc = new DOMParser().parseFromString(xml, "image/svg+xml");
  const svg = doc.querySelector("svg");
  if (!svg) return null;
  return {
    viewBox: svg.getAttribute("viewBox") || "0 0 820 320",
    body: svg.innerHTML,
  };
}

export async function loadAndAnimate(
  lang: string,
  container: HTMLDivElement | null
) {
  if (!container) return;
  let xml = HELLO_SVGS[lang] || HELLO_SVGS["en"];
  let parsed = extractSvg(xml);
  if (!parsed) {
    xml = HELLO_SVGS["en"];
    parsed = extractSvg(xml);
  }
  if (!parsed) return;

  const { viewBox, body } = parsed;

  container.innerHTML = `
    <svg class="hello__svg" viewBox="${viewBox}" xmlns="http://www.w3.org/2000/svg">
      ${body}
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

    path.style.stroke = "white";
    path.style.fill = "none";
    path.style.strokeWidth = "32px";

    const len = path.getTotalLength();
    const duration = len > 0 ? (len / totalLength) * totalDuration : 100;

    path.style.strokeDasharray = String(len);
    path.style.strokeDashoffset = String(len);
    if (len > 0) {
      path.animate([{ strokeDashoffset: len }, { strokeDashoffset: 0 }], {
        duration,
        delay: currentTime,
        fill: "forwards",
        easing: "linear",
      });
      currentTime += duration;
    }
  });
}