import { api } from "./api";
import { focusEngine } from "./focus/focus-engine";
import { applyTheme } from "./palette";
import { sound } from "./sound";
import { actions, selectTab, store, type TabId } from "./state";
import type { AppInfo, MediaItem } from "./types";
import "./styles/index.css";
import "./styles/volume-osd.css";
import "./styles/iptv.css";
import { el } from "./ui/icons";
import { showDialog } from "./ui/dialog";
import { renderHero, setHeroForElement, startHeroRotation } from "./ui/hero";
import { initGamepad } from "./gamepad";
import { renderHints } from "./ui/hints";
import { closeAllOverlays, closeTopOverlay, hasOverlay, toast } from "./ui/overlay";
import { openSearch, isSearchOpen, handleSearchKey } from "./ui/search";
import { openControlCentre } from "./ui/control-centre";
import { openSettings } from "./ui/settings";
import { startSplash, hideSplash } from "./ui/splash";
import { renderShelves } from "./ui/shelves";
import { launchApp, mediaFeedback, openMedia, playOnPicker } from "./ui/tiles";
import { renderTopbar, startClock } from "./ui/topbar";
import { initAod, refreshAod } from "./ui/aod";
import { initVolumeOsd } from "./ui/volume-osd";
import { loadIptvChannels, toggleIptvFavorite } from "./ui/iptv";

/** Repaint every chrome region after settings or data changed. */
function refreshChrome(rebuildFocus = true): void {
  applyTheme();
  renderTopbar();
  renderHero();
  renderShelves();
  renderHints();
  refreshAod();
  if (rebuildFocus) {
    focusEngine.rebuild();
  }
}

/** Details sheet for an app tile (I key). */
function openAppInfo(app: AppInfo): void {
  showDialog({
    title: app.name,
    body: "Launch this application?",
    returnKey: `tile-${app.id}`,
    actions: [
      { label: "Open", primary: true, onSelect: () => launchApp(app) },
      { label: "Cancel", onSelect: () => undefined },
    ],
  });
}

/** Details sheet for a recommended title (I key). */
function openTitleInfo(item: MediaItem): void {
  showDialog({
    title: item.title,
    body: [item.kind, item.year, item.stars].filter((part) => part && part.length > 0).join(" · "),
    returnKey: `tile-${item.id}`,
    actions: [
      { label: "Open", primary: true, onSelect: () => openMedia(item) },
      { label: "Play on…", onSelect: () => playOnPicker(item, `tile-${item.id}`) },
      { label: "Like", onSelect: () => void actions.feedback(item, "like") },
      { label: "Hide", onSelect: () => void actions.feedback(item, "hide") },
      { label: "Cancel", onSelect: () => undefined },
    ],
  });
}

/** Show the details sheet for whatever tile is focused. */
function infoForFocused(): void {
  const focused = document.activeElement as HTMLElement | null;
  const appId = focused?.closest<HTMLElement>("[data-app-id]")?.dataset.appId;
  if (appId) {
    const app = store.state.apps.find((entry) => entry.id === appId);
    if (app) openAppInfo(app);
    return;
  }
  const mediaId = focused?.closest<HTMLElement>("[data-media-id]")?.dataset.mediaId;
  if (mediaId) {
    const item = [...store.state.recommendations, ...store.state.catalog].find(
      (entry) => entry.id === mediaId,
    );
    if (item) openTitleInfo(item);
  }
}

/** Toggle the system mute state and mirror it into the store. */
async function toggleMute(): Promise<void> {
  try {
    await api.audioCommand("volume-mute");
    const [volume, muted] = await api.getAudio();
    store.set({ audio: { volume, muted } });
  } catch {
    /* the backend toasts failures itself */
  }
}

/** Toggle favorite for whatever tile is currently focused */
function toggleFavoriteForFocused(): void {
  const focused = focusEngine.focused;
  if (!focused) return;
  const appId = focused.dataset.appId;
  if (appId) {
    const app = store.state.apps.find((a) => a.id === appId);
    if (app) {
      const isFav = store.state.settings.favorites.includes(app.id);
      const favorites = isFav
        ? store.state.settings.favorites.filter((id) => id !== app.id)
        : [...store.state.settings.favorites, app.id];
      actions.patchSettings({ favorites });
      toast(isFav ? `${app.name} removed from favorites` : `${app.name} added to favorites ⭐`, "ok");
      renderShelves();
      renderHero();
      return;
    }
  }
  const channelId = focused.dataset.channelId;
  if (channelId) {
    toggleIptvFavorite(channelId);
    renderShelves();
    renderHero();
    return;
  }
  const mediaId = focused.dataset.mediaId;
  if (mediaId) {
    const item = [...store.state.recommendations, ...store.state.catalog].find((m) => m.id === mediaId);
    if (item) {
      const liked = store.state.profile.liked.includes(item.id);
      mediaFeedback(item, liked ? "hide" : "like", liked ? "Removed from favorites" : "Added to favorites ⭐");
      renderShelves();
      renderHero();
      return;
    }
  }
}

let enterKeyTimer: number | null = null;
let enterKeyLongTriggered = false;

function onKeydown(event: KeyboardEvent): void {
  const key = event.key;

  // Home key / Super key returns to home screen and closes overlays
  if (key === "Home" || key === "Meta" || key === "Super") {
    closeAllOverlays();
    selectTab("home");
    refreshChrome();
    focusEngine.focusFirst("topbar");
    event.preventDefault();
    return;
  }

  if (event.metaKey || event.ctrlKey || event.altKey) return;

  // While search is open, printable keys and Backspace type into the field
  // instead of doing anything else.
  if (hasOverlay() && isSearchOpen() && handleSearchKey(key)) {
    event.preventDefault();
    return;
  }

  // Back / close always works, overlay or not.
  if (key === "Escape" || key === "Backspace") {
    if (hasOverlay()) closeTopOverlay();
    else sound.back();
    event.preventDefault();
    return;
  }

  // Ignore key repeat for activation keys to prevent multiple triggers
  if (event.repeat && (key === "Enter" || key === "f" || key === "F" || key === "x" || key === "X")) {
    event.preventDefault();
    return;
  }

  if (hasOverlay()) {
    switch (key) {
      case "ArrowUp":
        focusEngine.move("up");
        break;
      case "ArrowDown":
        focusEngine.move("down");
        break;
      case "ArrowLeft":
        focusEngine.move("left");
        break;
      case "ArrowRight":
        focusEngine.move("right");
        break;
      case "Enter":
        focusEngine.activate();
        break;
      default:
        return;
    }
    event.preventDefault();
    return;
  }

  switch (key) {
    case "ArrowUp":
      if (focusEngine.move("up")) sound.focus();
      break;
    case "ArrowDown":
      if (focusEngine.move("down")) sound.focus();
      break;
    case "ArrowLeft":
      if (focusEngine.move("left")) sound.focus();
      break;
    case "ArrowRight":
      if (focusEngine.move("right")) sound.focus();
      break;
    case "Enter": {
      const focused = focusEngine.focused;
      const isTile = focused && (focused.dataset.appId || focused.dataset.channelId || focused.dataset.mediaId);
      if (isTile) {
        enterKeyLongTriggered = false;
        if (enterKeyTimer) window.clearTimeout(enterKeyTimer);
        enterKeyTimer = window.setTimeout(() => {
          enterKeyLongTriggered = true;
          const handler = focused ? focusEngine.handlersFor(focused) : undefined;
          if (handler?.onContext) {
            handler.onContext();
          }
        }, 340);
      } else {
        if (focusEngine.activate()) sound.select();
      }
      break;
    }
    case "f":
    case "F":
      toggleFavoriteForFocused();
      break;
    case "x":
    case "X":
    case "ContextMenu": {
      const focused = focusEngine.focused;
      const handler = focused ? focusEngine.handlersFor(focused) : undefined;
      if (handler?.onContext) handler.onContext();
      break;
    }
    case "i":
    case "I":
      infoForFocused();
      break;
    case "m":
    case "M":
      void toggleMute();
      break;
    case "s":
    case "S":
      openSettings();
      break;
    case "c":
    case "C":
      openControlCentre();
      break;
    default:
      return;
  }
  event.preventDefault();
}

function onKeyup(event: KeyboardEvent): void {
  if (event.key === "Enter") {
    if (enterKeyTimer) {
      window.clearTimeout(enterKeyTimer);
      enterKeyTimer = null;
    }
    if (!enterKeyLongTriggered && !hasOverlay()) {
      const focused = focusEngine.focused;
      const isTile = focused && (focused.dataset.appId || focused.dataset.channelId || focused.dataset.mediaId);
      if (isTile) {
        if (focusEngine.activate()) sound.select();
      }
    }
    enterKeyLongTriggered = false;
  }
}

function wireEvents(): void {
  document.addEventListener("keydown", onKeydown);
  document.addEventListener("keyup", onKeyup);

  // Reactively apply settings and theme whenever store changes
  store.subscribe(() => {
    applyTheme();
    renderHints();
  });

  // The Top Shelf mirrors whatever tile has focus (hero.ts owns the content).
  focusEngine.onchange((element) => setHeroForElement(element));

  document.addEventListener("launcher:home", () => {
    closeAllOverlays();
    selectTab("home");
    refreshChrome();
    focusEngine.focusFirst("topbar");
  });

  document.addEventListener("launcher:tab", (event) => {
    const id = (event as CustomEvent<TabId>).detail;
    if (id === "search") {
      openSearch();
      return;
    }
    if (id === "settings") {
      openSettings();
      return;
    }
    selectTab(id);
    refreshChrome();
    focusEngine.focusFirst("topbar");
  });

  document.addEventListener("launcher:tab-preview", (event) => {
    const id = (event as CustomEvent<TabId>).detail;
    selectTab(id);
    // Refresh content below without shifting focus away from the active tab button
    renderShelves();
    renderHero();
  });

  // The brand hero offers a "Open Settings" pill.
  document.addEventListener("launcher:open-settings", () => openSettings());
}

async function boot(): Promise<void> {
  // The 10s splash sits on top while the app preloads its data, so the home
  // screen paints fully (and the media catalogue finishes fetching) behind it.
  const splash = startSplash();
  await Promise.all([actions.bootstrap(), splash]);
  hideSplash();
  wireEvents();
  initVolumeOsd();
  
  // Background fetch country IPTV channels
  void loadIptvChannels()
    .then(() => {
      renderShelves();
    })
    .catch(() => undefined);

  refreshChrome();
  focusEngine.focusFirst("topbar");
  startClock();
  startHeroRotation();
  initGamepad();
  initAod();

  await api
    .onAppsScanned((apps) => {
      store.set({ apps });
      refreshChrome();
    })
    .catch(() => undefined);
  await api
    .onLaunchResult((result) => {
      if (!result.ok) sound.error();
    })
    .catch(() => undefined);
}

/** Last-resort paint if booting fails: never leave a black screen. */
function showBootError(error: unknown): void {
  console.error("boot failed", error);
  hideSplash();
  const stage = document.getElementById("rows");
  const topbar = document.getElementById("topbar");
  if (topbar) topbar.replaceChildren();
  if (stage) {
    const card = el("div", "boot-error");
    card.appendChild(el("h1", undefined, "The launcher could not start"));
    card.appendChild(el("p", undefined, String(error)));
    stage.replaceChildren(card);
  }
}

boot().catch(showBootError);
