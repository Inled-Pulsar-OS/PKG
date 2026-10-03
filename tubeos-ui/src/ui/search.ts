import { api } from "../api";
import { focusEngine } from "../focus/focus-engine";
import { sound } from "../sound";
import { store } from "../state";
import type { AppInfo, MediaItem } from "../types";
import { appTile, mediaTile } from "./tiles";
import { el, icon } from "./icons";
import { closeAllOverlays, openOverlay } from "./overlay";

/**
 * The tvOS Search screen: an on-screen keyboard, live results and a field that
 * also accepts hardware keys. Results search two things, like a real Apple TV:
 * installed apps and the IMDb movie / TV catalogue (via `media_search`), laid
 * out as separate "Apps" and "Movies & TV" sections.
 */
let query = "";
let inputEl: HTMLElement | null = null;
let appsEl: HTMLElement | null = null;
let appsSection: HTMLElement | null = null;
let mediaEl: HTMLElement | null = null;
let mediaSection: HTMLElement | null = null;
let emptyEl: HTMLElement | null = null;
let debounce = 0;

/** True while the search overlay is the active layer (for hardware typing). */
export function isSearchOpen(): boolean {
  return focusEngine.activeLayer === "search";
}

/** Handle a physical key while search is open; returns true if consumed. */
export function handleSearchKey(key: string): boolean {
  if (key === "Backspace") {
    if (query.length === 0) return false;
    query = query.slice(0, -1);
  } else if (key.length === 1) {
    query += key.toUpperCase();
  } else {
    return false;
  }
  sound.select();
  paintField();
  refresh();
  return true;
}

function paintField(): void {
  if (!inputEl) return;
  inputEl.replaceChildren();
  if (query.length === 0) {
    inputEl.appendChild(el("span", "search__placeholder", "Type to search…"));
  } else {
    inputEl.appendChild(document.createTextNode(query));
  }
  inputEl.appendChild(el("span", "caret"));
}

async function runSearch(): Promise<void> {
  const text = query.trim();
  const [apps, media] = await Promise.all([
    api.searchApps(text, 12),
    api.mediaSearch(text, 16).catch(() => [] as MediaItem[]),
  ]);
  paintResults(apps, media);
}

function paintResults(apps: AppInfo[], media: MediaItem[]): void {
  if (!appsEl || !mediaEl || !emptyEl) return;
  appsEl.replaceChildren();
  mediaEl.replaceChildren();

  const any = apps.length > 0 || media.length > 0;
  emptyEl.hidden = any;

  if (apps.length > 0 && appsSection) {
    appsSection.hidden = false;
    for (const app of apps) appsEl.appendChild(appTile(app, `search-app-${app.id}`));
    focusEngine.registerZone("search-apps", appsEl, 1);
  } else {
    if (appsSection) appsSection.hidden = true;
    focusEngine.unregisterZone("search-apps");
  }

  if (media.length > 0 && mediaSection) {
    mediaSection.hidden = false;
    for (const item of media) mediaEl.appendChild(mediaTile(item, `search-media-${item.id}`));
    focusEngine.registerZone("search-media", mediaEl, 2);
  } else {
    if (mediaSection) mediaSection.hidden = true;
    focusEngine.unregisterZone("search-media");
  }
}

function refresh(): void {
  window.clearTimeout(debounce);
  if (query.trim().length === 0) {
    emptyEl?.toggleAttribute("hidden", true);
    if (appsEl) {
      appsEl.replaceChildren();
      if (appsSection) appsSection.hidden = false;
      if (mediaSection) mediaSection.hidden = true;
    }
    if (mediaEl) mediaEl.replaceChildren();
    focusEngine.unregisterZone("search-apps");
    focusEngine.unregisterZone("search-media");
    return;
  }
  debounce = window.setTimeout(() => void runSearch(), 220);
}

import { VirtualKeyboard } from "./keyboard";

let activeKeyboard: VirtualKeyboard | null = null;

function buildKeyboard(wrap: HTMLElement): HTMLElement {
  activeKeyboard = new VirtualKeyboard({
    container: wrap,
    initialValue: query,
    onInput: (newVal) => {
      query = newVal;
      paintField();
      refresh();
    },
    onSubmit: () => {
      // If there are results, focus first app or media
      const firstApp = appsEl?.querySelector<HTMLElement>(".focusable");
      const firstMedia = mediaEl?.querySelector<HTMLElement>(".focusable");
      if (firstApp) focusEngine.focusElement(firstApp);
      else if (firstMedia) focusEngine.focusElement(firstMedia);
    },
    onClose: () => {
      // User moved UP past top row or pressed Back -> focus search field or results
      const firstApp = appsEl?.querySelector<HTMLElement>(".focusable");
      if (firstApp) focusEngine.focusElement(firstApp);
      else focusEngine.rebuild("tab-search");
    }
  });

  return activeKeyboard.render();
}

export function openSearch(): void {
  closeAllOverlays();
  query = "";
  openOverlay({
    layer: "search",
    className: "overlay--search",
    build: (root) => {
      const wrap = el("div", "search");
      const field = el("div", "search__field");
      field.appendChild(icon("search", 21));
      inputEl = el("div", "search__input");
      paintField();
      field.appendChild(inputEl);
      wrap.appendChild(field);

      const body = el("div", "search__body");
      appsSection = el("div", "search__section");
      appsSection.hidden = true;
      appsSection.appendChild(el("h3", undefined, "Apps"));
      appsEl = el("div", "result-grid");
      appsSection.appendChild(appsEl);
      body.appendChild(appsSection);

      mediaSection = el("div", "search__section");
      mediaSection.hidden = true;
      mediaSection.appendChild(el("h3", undefined, "Movies & TV"));
      mediaEl = el("div", "result-grid");
      mediaSection.appendChild(mediaEl);
      body.appendChild(mediaSection);

      emptyEl = el("div", "search__empty", "No matches. Try another spelling.");
      emptyEl.hidden = true;
      body.appendChild(emptyEl);
      wrap.appendChild(body);

      wrap.appendChild(buildKeyboard(wrap));
      root.appendChild(wrap);
    },
    returnKey: "tab-search",
    onClose: () => {
      window.clearTimeout(debounce);
      query = "";
      activeKeyboard?.close();
      activeKeyboard = null;
      focusEngine.rebuild(`tab-${store.state.tab}`);
    },
  });
}