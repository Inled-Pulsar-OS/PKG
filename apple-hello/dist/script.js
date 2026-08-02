const DEFAULT_LANG = "en"; // Set to null to use browser language

const SUPPORTED = [
  "ar","bg","ca","cs","da","de","el","en","es","fi","fr",
  "he","hi","hr","hu","id","it","ja","kk","ko","ms","nb",
  "nl","pl","pt","ro","ru","sk","sv","th","tr","uk","vi",
  "zh"
];

function getLang() {
  if (DEFAULT_LANG) return DEFAULT_LANG;
  const code = navigator.language.slice(0, 2);
  return SUPPORTED.includes(code) ? code : "en";
}

async function loadSVG(lang) {
  const res = await fetch(`./svg/hello-${lang}.svg`);
  const text = await res.text();
  const parser = new DOMParser();
  const doc = parser.parseFromString(text, "image/svg+xml");
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
  const lang = getLang();
  const svg = await loadSVG(lang);
  const container = document.getElementById("hello");
  container.setAttribute("viewBox", svg.getAttribute("viewBox"));
  container.innerHTML = svg.innerHTML;
  animatePaths();
}

init();
