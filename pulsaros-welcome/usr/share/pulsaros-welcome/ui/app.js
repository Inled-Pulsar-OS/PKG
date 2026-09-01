// ==============================================================================
// Pulsar OS Welcome Frontend State Machine & Animations
// ==============================================================================

const IS_LIVE = window.IS_LIVE !== undefined ? window.IS_LIVE : false;
const NEEDS_OOTB = window.NEEDS_OOTB !== undefined ? window.NEEDS_OOTB : false;

const SLIDES = ['hello', 'features', 'compatibility', 'settings', 'sayri'];
if (IS_LIVE) {
  SLIDES.push('recovery');
}
SLIDES.push('done');

let currentSlideIndex = 0;

// Features Carousel Data
const CAROUSEL_DATA = [
  {
    title: "Intelligent Window Snapping",
    subtitle: "Organize your workflow effortlessly with magnetic window tiling, intuitive edge-snapping, and smooth keyboard shortcuts.",
    video: "../public/videos/window-mode.mp4",
  },
  {
    title: "Native AI Integrations",
    subtitle: "Work with leading artificial intelligence models directly from your desktop. Integrated with Claude, ChatGPT, DeepSeek, Gemini, Perplexity, and OpenClaw.",
    providers: [
      { name: "Claude", src: "../public/logos/providers/claude.png" },
      { name: "ChatGPT", src: "../public/logos/providers/chatgpt.png" },
      { name: "DeepSeek", src: "../public/logos/providers/deepseek.png" },
      { name: "Gemini", src: "../public/logos/providers/gemini.png" },
      { name: "Perplexity", src: "../public/logos/providers/perplexity.png" },
      { name: "OpenClaw", src: "../public/logos/providers/openclaw.png" }
    ]
  },
  {
    title: "Universal Desktop Search",
    subtitle: "Find apps, documents, cloud files, settings, and web answers instantly with Spotlight search.",
    video: "../public/videos/spotlight.mp4",
  },
  {
    title: "Session Restore & Live Wallpaper",
    subtitle: "Pick up right where you left off with intelligent workspace restoration and dynamic interactive backgrounds.",
    video: "../public/videos/remap-live-wallpaper.mp4",
  }
];

let carouselIndex = 0;
let carouselTimer = null;
const CAROUSEL_INTERVAL = 9000;

// Backend Python bridge caller
function callBackend(action, payload = {}) {
  const data = JSON.stringify({ action: action, ...payload });
  if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.welcome) {
    window.webkit.messageHandlers.welcome.postMessage(data);
  } else {
    console.log("[Backend Call]", action, payload);
  }
}

// ------------------------------------------------------------------------------
// Navigation Controller
// ------------------------------------------------------------------------------
function updateScreen() {
  const currentName = SLIDES[currentSlideIndex];
  
  // Hide all screens
  document.querySelectorAll('.screen-container, .screen-backdrop').forEach(el => {
    el.classList.remove('active');
  });

  const activeEl = document.getElementById(`screen-${currentName}`);
  if (activeEl) {
    activeEl.classList.add('active');
  }

  if (currentName === 'features') {
    startCarousel();
  } else {
    stopCarousel();
  }

  if (currentName === 'done') {
    initSvgAnimation('done-svg');
    callBackend('write_sentinel');
  }
}

function nextSlide() {
  if (currentSlideIndex < SLIDES.length - 1) {
    currentSlideIndex++;
    updateScreen();
  }
}

function prevSlide() {
  if (currentSlideIndex > 0) {
    currentSlideIndex--;
    updateScreen();
  }
}

function onHelloContinue() {
  if (NEEDS_OOTB) {
    callBackend('launch_ootb');
    return;
  }
  nextSlide();
}

function onDoneFinish() {
  callBackend('close');
}

// ------------------------------------------------------------------------------
// Carousel Controller
// ------------------------------------------------------------------------------
function renderCarouselSlide() {
  const data = CAROUSEL_DATA[carouselIndex];
  document.getElementById('carousel-title').innerText = data.title;
  document.getElementById('carousel-subtitle').innerText = data.subtitle;

  const dots = document.querySelectorAll('#carousel-dots .carousel-dot');
  dots.forEach((dot, idx) => {
    dot.classList.toggle('active', idx === carouselIndex);
  });

  const body = document.getElementById('carousel-body');
  if (data.providers) {
    let chips = data.providers.map(p => `
      <div class="provider-chip">
        <img src="${p.src}" alt="${p.name}">
        <span>${p.name}</span>
      </div>
    `).join('');
    body.innerHTML = `
      <div style="position: relative; width: 100%; max-width: 620px; min-height: 240px; display: flex; align-items: center; justify-content: center;">
        <div class="providers-grid">${chips}</div>
        <button class="carousel-nav-btn carousel-nav-prev" onclick="prevCarouselSlide()">&#10094;</button>
        <button class="carousel-nav-btn carousel-nav-next" onclick="nextCarouselSlide()">&#10095;</button>
      </div>
    `;
  } else if (data.video) {
    body.innerHTML = `
      <div class="slide-video-box">
        <video src="${data.video}" autoplay muted loop playsinline></video>
        <button class="carousel-nav-btn carousel-nav-prev" onclick="prevCarouselSlide()">&#10094;</button>
        <button class="carousel-nav-btn carousel-nav-next" onclick="nextCarouselSlide()">&#10095;</button>
      </div>
    `;
  }

  // Reset progress bar
  const bar = document.getElementById('carousel-progress-bar');
  if (bar) {
    bar.style.transition = 'none';
    bar.style.width = '0%';
    setTimeout(() => {
      bar.style.transition = `width ${CAROUSEL_INTERVAL}ms linear`;
      bar.style.width = '100%';
    }, 50);
  }
}

function nextCarouselSlide() {
  carouselIndex = (carouselIndex + 1) % CAROUSEL_DATA.length;
  renderCarouselSlide();
}

function prevCarouselSlide() {
  carouselIndex = (carouselIndex - 1 + CAROUSEL_DATA.length) % CAROUSEL_DATA.length;
  renderCarouselSlide();
}

function goToCarouselSlide(idx) {
  carouselIndex = idx;
  renderCarouselSlide();
  resetCarouselTimer();
}

function startCarousel() {
  renderCarouselSlide();
  resetCarouselTimer();
}

function stopCarousel() {
  if (carouselTimer) clearInterval(carouselTimer);
}

function resetCarouselTimer() {
  stopCarousel();
  carouselTimer = setInterval(nextCarouselSlide, CAROUSEL_INTERVAL);
}

// ------------------------------------------------------------------------------
// SVG Animation (Hello & Done)
// ------------------------------------------------------------------------------
const SUPPORTED_LANGS = [
  "ar","bg","ca","cs","da","de","el","en","es","fi","fr",
  "he","hi","hr","hu","id","it","ja","kk","ko","ms","nb",
  "nl","pl","pt","pt_BR","ro","ru","sk","sv","th","tr","uk","vi",
  "zh_HK", "zh_Hans", "zh_Hant"
];

function getSystemLanguage() {
  const params = new URLSearchParams(window.location.search);
  const forced = params.get("lang");
  if (forced && SUPPORTED_LANGS.includes(forced)) return forced;

  const nav = (navigator.language || 'en').replace('-', '_');
  if (SUPPORTED_LANGS.includes(nav)) return nav;
  const shortCode = nav.slice(0, 2);
  if (SUPPORTED_LANGS.includes(shortCode)) return shortCode;
  return 'en';
}

async function initSvgAnimation(elementId) {
  const lang = getSystemLanguage();
  const svgEl = document.getElementById(elementId);
  if (!svgEl) return;

  let candidates = [`../hello/svg/hello-${lang}.svg`, `../hello/svg/hello-en.svg`];
  let svgText = null;

  for (const path of candidates) {
    try {
      const res = await fetch(path);
      if (res.ok) {
        svgText = await res.text();
        break;
      }
    } catch (e) {}
  }

  if (!svgText) return;

  const parser = new DOMParser();
  const doc = parser.parseFromString(svgText, "image/svg+xml");
  const parsedSvg = doc.querySelector("svg");
  if (!parsedSvg) return;

  svgEl.innerHTML = parsedSvg.innerHTML;
  const vb = (parsedSvg.getAttribute("viewBox") || "0 0 900 300").split(/[ ,]+/).map(Number);
  if (vb.length === 4) {
    svgEl.setAttribute("viewBox", `${vb[0]} ${vb[1]} ${vb[2]} ${vb[3]}`);
  }

  // Animate paths sequentially
  const paths = svgEl.querySelectorAll("path");
  let totalLength = 0;
  paths.forEach(p => totalLength += p.getTotalLength());

  const totalDuration = 4000;
  let delay = 0;

  paths.forEach(p => {
    p.removeAttribute("stroke");
    p.removeAttribute("stroke-width");
    p.removeAttribute("fill");
    
    const len = p.getTotalLength();
    const duration = (len / (totalLength || 1)) * totalDuration;

    p.style.stroke = "#ffffff";
    p.style.strokeWidth = "48px";
    p.style.fill = "none";
    p.style.strokeLinecap = "round";
    p.style.strokeDasharray = len;
    p.style.strokeDashoffset = len;

    p.animate([
      { strokeDashoffset: len },
      { strokeDashoffset: 0 }
    ], {
      duration: duration,
      delay: delay,
      fill: "forwards",
      easing: "ease-in-out"
    });

    delay += duration;
  });
}

// ------------------------------------------------------------------------------
// Initialization
// ------------------------------------------------------------------------------
document.addEventListener("DOMContentLoaded", () => {
  initSvgAnimation('hello-svg');
});
