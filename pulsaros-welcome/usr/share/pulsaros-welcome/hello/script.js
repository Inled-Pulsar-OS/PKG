const DEFAULT_LANG = null;

const SUPPORTED = [
  "ar","bg","ca","cs","da","de","el","en","es","fi","fr",
  "he","hi","hr","hu","id","it","ja","kk","ko","ms","nb",
  "nl","pl","pt","pt_BR","ro","ru","sk","sv","th","tr","uk","vi",
  "zh_HK", "zh_Hans", "zh_Hant"
];

function getLang() {
  if (DEFAULT_LANG) {
    return SUPPORTED.includes(DEFAULT_LANG) ? DEFAULT_LANG : "en";
  }
  const forced = new URLSearchParams(location.search).get("lang");
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

async function loadSVG(lang) {
  const res = await fetch(`./svg/hello-${lang}.svg`);
  const text = await res.text();
  const doc = new DOMParser().parseFromString(text, "image/svg+xml");
  return doc.querySelector("svg");
}

function stripInlineStyles(el) {
  el.removeAttribute("stroke");
  el.removeAttribute("stroke-width");
  el.removeAttribute("stroke-linecap");
  el.removeAttribute("stroke-miterlimit");
  el.removeAttribute("fill");
}

function animatePaths() {
  const paths = document.querySelectorAll("#hello path");
  const totalDuration = 5000;

  let totalLength = 0;
  paths.forEach(path => totalLength += path.getTotalLength());

  let currentTime = 0;
  paths.forEach((path) => {
    stripInlineStyles(path);
    const len = path.getTotalLength();
    const duration = (len / totalLength) * totalDuration;

    path.style.strokeDasharray = len;
    path.style.strokeDashoffset = len;
    path.animate([
      { strokeDashoffset: len },
      { strokeDashoffset: 0 }
    ], {
      duration,
      delay: currentTime,
      fill: "forwards",
      easing: "linear"
    });
    currentTime += duration;
  });
}

async function init() {
  let lang = getLang();
  let svg = null;
  for (const candidate of [lang, "en"]) {
    try {
      svg = await loadSVG(candidate);
      break;
    } catch (err) {
      console.error(`Unable to load hello SVG for "${candidate}"`);
    }
  }
  if (!svg) return;

  const vb = (svg.getAttribute("viewBox") || "").split(/[ ,]+/).map(Number);
  const container = document.getElementById("hello");
  container.innerHTML = svg.innerHTML;

  if (vb.length === 4) {
    const path = container.querySelector("path");
    const strokeWidth = path ? parseFloat(getComputedStyle(path).strokeWidth) : 48;
    const bleed = (strokeWidth || 48) / 2;
    container.setAttribute("viewBox",
      `${vb[0] - bleed} ${vb[1] - bleed} ${vb[2] + bleed * 2} ${vb[3] + bleed * 2}`);
  }

  animatePaths();
}

init();
