import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, iconUrl, mediaMeta, posterUrl } from "../api";
import { focusEngine, makeFocusable } from "../focus/focus-engine";
import { washFor } from "../palette";
import { sound } from "../sound";
import { actions, store } from "../state";
import type { AppInfo, MediaItem } from "../types";
import { el, icon } from "./icons";
import { showDialog, showPicker } from "./dialog";
import { showActionSheet } from "./menu";
import { openOverlay, toast } from "./overlay";

/** Parallax the artwork slightly while the pointer travels over a tile. */
function addParallax(art: HTMLElement): void {
  art.addEventListener("pointermove", (event) => {
    if (!store.state.settings.parallax) return;
    const rect = art.getBoundingClientRect();
    const x = ((event.clientX - rect.left) / rect.width - 0.5) * 12;
    const y = ((event.clientY - rect.top) / rect.height - 0.5) * 12;
    art.style.setProperty("--px", `${x.toFixed(2)}px`);
    art.style.setProperty("--py", `${y.toFixed(2)}px`);
  });
  art.addEventListener("pointerleave", () => {
    art.style.setProperty("--px", "0px");
    art.style.setProperty("--py", "0px");
  });
}

/** Build the rounded artwork square: image on top of a gradient fallback. */
function artWithImage(src: string | null, fallback: HTMLElement | null, alt: string): HTMLElement {
  const art = el("div", "tile-art");
  if (src) {
    const img = el("img");
    img.src = src;
    img.alt = alt;
    img.loading = "lazy";
    img.decoding = "async";
    img.addEventListener("error", () => {
      img.remove();
      art.appendChild(fallback || el("div", "tile-art__fallback", initialsOf(alt)));
    });
    art.appendChild(img);
  } else {
    art.appendChild(fallback || el("div", "tile-art__fallback", initialsOf(alt)));
  }
  addParallax(art);
  return art;
}

/** Helper to detect long press / hold on any element and trigger action */
export function attachLongPress(element: HTMLElement, callback: () => void, delayMs = 400): void {
  let timer: number | null = null;
  let didLongPress = false;

  const start = () => {
    didLongPress = false;
    if (timer) window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      didLongPress = true;
      callback();
    }, delayMs);
  };

  const clear = () => {
    if (timer) {
      window.clearTimeout(timer);
      timer = null;
    }
  };

  element.addEventListener("pointerdown", start);
  element.addEventListener("pointerup", clear);
  element.addEventListener("pointerleave", clear);
  element.addEventListener("pointercancel", clear);

  element.addEventListener(
    "click",
    (e) => {
      if (didLongPress) {
        e.preventDefault();
        e.stopPropagation();
        didLongPress = false;
      }
    },
    { capture: true },
  );
}

function initialsOf(name: string): string {
  const words = name.replace(/[^\p{L}\p{N} ]+/gu, " ").split(/\s+/).filter(Boolean);
  if (!words.length) return "?";
  if (words.length === 1) return words[0].slice(0, 2).toUpperCase();
  return `${words[0][0]}${words[1][0]}`.toUpperCase();
}

function tileBadge(iconName: string, extraClass = ""): HTMLElement {
  const node = el("div", `tile-badge${extraClass ? ` ${extraClass}` : ""}`);
  node.appendChild(icon(iconName, 12));
  return node;
}

/** Paint the backdrop wash for the focused item. */
function setWash(seed: string): void {
  const { a, b } = washFor(seed);
  document.body.style.setProperty("--wash-a", a);
  document.body.style.setProperty("--wash-b", b);
}

/** Launch a real application, honouring the confirm/hide settings. */
export function launchApp(app: AppInfo): void {
  const settings = store.state.settings;
  const run = (): void => {
    sound.launch();
    void api
      .launchApp(app.id)
      .then(() => {
        const apps = { ...store.state.usage.apps };
        const entry = apps[app.id] ?? { count: 0, lastUsed: 0 };
        apps[app.id] = { count: entry.count + 1, lastUsed: Math.floor(Date.now() / 1000) };
        store.set({ usage: { apps } });
        toast(`Opening ${app.name}`, "ok");
        if (settings.hideOnLaunch) {
          window.setTimeout(() => {
            void getCurrentWindow().hide();
          }, 850);
        }
      })
      .catch((error: unknown) => {
        sound.error();
        toast(String(error), "error");
      });
  };

  if (settings.confirmLaunch) {
    showDialog({
      title: `Open ${app.name}?`,
      body: app.comment ?? app.genericName ?? "This will start the application.",
      actions: [
        { label: "Cancel", onSelect: () => undefined },
        { label: "Open", primary: true, onSelect: run },
      ],
      returnKey: `tile-${app.id}`,
    });
    return;
  }
  run();
}

/** One app tile, styled like a tvOS home screen icon. */
export function appTile(app: AppInfo, focusKey: string): HTMLElement {
  const tile = el("button", "tile");
  tile.dataset.appId = app.id;
  const src = iconUrl(app.iconName, app.iconPath);
  const fallback = el("div", "tile-art__fallback", initialsOf(app.name));
  tile.appendChild(artWithImage(src, fallback, app.name));

  const isFavorite = store.state.settings.favorites.includes(app.id);
  const isHidden = store.state.settings.hiddenApps.includes(app.id);
  if (isFavorite || isHidden) {
    const badges = el("div", "tile-badges");
    if (isFavorite) badges.appendChild(tileBadge("star", "tile-badge--like"));
    if (isHidden) badges.appendChild(tileBadge("eyeSlash"));
    tile.appendChild(badges);
  }

  if (store.state.settings.showLabels) {
    tile.appendChild(el("div", "tile-label", app.name));
  }
  if (isHidden) tile.classList.add("is-hidden-app");

  attachLongPress(tile, () => {
    openAppMenu(app, tile);
  });

  makeFocusable(
    tile,
    {
      onFocus: () => {
        sound.focus();
        setWash(app.id);
      },
      onActivate: () => launchApp(app),
      onContext: () => openAppMenu(app, tile),
    },
    focusKey,
  );
  return tile;
}

/** Learn from a poster interaction and confirm it visually. */
export function mediaFeedback(item: MediaItem, action: string, label: string): void {
  sound.toggle();
  void actions.feedback(item, action).then(() => toast(label, "ok"));
}

/** One IMDb poster tile. */
export function mediaTile(item: MediaItem, focusKey: string): HTMLElement {
  const tile = el("button", "tile tile--poster");
  tile.dataset.mediaId = item.id;
  const src = posterUrl(item);
  const { a, b } = washFor(item.genre || item.title);
  const fallback = el("div", "poster-fallback");
  fallback.style.setProperty("--wash-a", a);
  fallback.style.setProperty("--wash-b", b);
  fallback.appendChild(el("span", undefined, item.title));
  tile.appendChild(artWithImage(src, fallback, item.title));

  const profile = store.state.profile;
  const liked = profile.liked.includes(item.id);
  const watched = profile.watched.includes(item.id);
  if (liked || watched) {
    const badges = el("div", "tile-badges");
    if (liked) badges.appendChild(tileBadge("heart", "tile-badge--like"));
    if (watched) badges.appendChild(tileBadge("check"));
    tile.appendChild(badges);
  }

  if (store.state.settings.showLabels) {
    tile.appendChild(el("div", "tile-label", item.title));
    tile.appendChild(el("div", "tile-sub", mediaMeta(item)));
  }

  attachLongPress(tile, () => {
    openMediaMenu(item, tile);
  });

  makeFocusable(
    tile,
    {
      onFocus: () => {
        sound.focus();
        setWash(item.genre || item.title);
      },
      onActivate: () => openMedia(item),
      onContext: () => openMediaMenu(item, tile),
    },
    focusKey,
  );
  return tile;
}

/** Open the IMDb page and record the click for the recommendation engine. */
export function openMedia(item: MediaItem): void {
  sound.select();
  void api.mediaOpen(item.id).catch((error: unknown) => {
    sound.error();
    toast(String(error), "error");
  });
  void actions.feedback(item, "click");
}

/** A streaming provider a poster can be searched on. */
export interface StreamingService {
  id: string;
  name: string;
  icon: string;
  /** Search URL for a title (opened in the desktop browser). */
  url: (title: string) => string;
}

/** "Play on…" targets shown for every IMDb title. */
export const STREAMING_SERVICES: StreamingService[] = [
  {
    id: "justwatch",
    name: "JustWatch (Where to Watch)",
    icon: "search",
    url: (title) => {
      const lang = (navigator.language || "es").toLowerCase();
      if (lang.startsWith("es")) {
        return `https://www.justwatch.com/es/buscar?q=${encodeURIComponent(title)}`;
      }
      return `https://www.justwatch.com/us/search?q=${encodeURIComponent(title)}`;
    },
  },
  {
    id: "netflix",
    name: "Netflix",
    icon: "play",
    url: (title) => `https://www.netflix.com/search?q=${encodeURIComponent(title)}`,
  },
  {
    id: "prime",
    name: "Amazon Prime Video",
    icon: "film",
    url: (title) => `https://www.primevideo.com/search/ref=atv_nb_sr?phrase=${encodeURIComponent(title)}`,
  },
  {
    id: "disney",
    name: "Disney+",
    icon: "star",
    url: (title) => `https://www.disneyplus.com/search/${encodeURIComponent(title)}`,
  },
  {
    id: "max",
    name: "Max (HBO)",
    icon: "tv",
    url: (title) => `https://play.max.com/search?q=${encodeURIComponent(title)}`,
  },
  {
    id: "apple",
    name: "Apple TV+",
    icon: "tv",
    url: (title) => `https://tv.apple.com/search?term=${encodeURIComponent(title)}`,
  },
  {
    id: "youtube",
    name: "YouTube",
    icon: "external",
    url: (title) => `https://www.youtube.com/results?search_query=${encodeURIComponent(title)}`,
  },
];

/** Search a title on a streaming service using the desktop's default handler. */
export function openStreaming(item: MediaItem, service: StreamingService): void {
  const url = service.url(item.title);
  sound.select();
  toast(`Opening on ${service.name}…`, "info");
  void api.openTarget(url).catch((error: unknown) => {
    sound.error();
    toast(`Could not open ${service.name}: ${String(error)}`, "error");
  });
}

/** The tvOS style picker listing the streaming services for a title. */
export function playOnPicker(item: MediaItem, returnKey?: string): void {
  showPicker({
    title: "Watch on…",
    subtitle: item.title,
    options: STREAMING_SERVICES.map((service) => ({
      label: service.name,
      value: service.id,
    })),
    current: "",
    onSelect: (id) => {
      const service = STREAMING_SERVICES.find((entry) => entry.id === id);
      if (service) openStreaming(item, service);
    },
    returnKey,
  });
}

/** tvOS long-press menu for an app tile. */
export function openAppMenu(app: AppInfo, _anchor?: HTMLElement): void {
  const settings = store.state.settings;
  const favorite = settings.favorites.includes(app.id);
  const hidden = settings.hiddenApps.includes(app.id);
  const src = iconUrl(app.iconName, app.iconPath);

  showActionSheet(
    {
      title: app.name,
      subtitle: app.genericName || app.group || "Application",
      iconSrc: src,
      iconName: "apps",
    },
    [
      {
        icon: "star",
        label: favorite ? "Remove Favorite" : "Add to Favorites",
        primary: true,
        action: () => {
          const favorites = favorite
            ? settings.favorites.filter((id) => id !== app.id)
            : [...settings.favorites, app.id];
          actions.patchSettings({ favorites });
          toast(favorite ? `${app.name} removed from favorites` : `${app.name} added to favorites ⭐`, "ok");
          document.dispatchEvent(new CustomEvent("launcher:tab-preview", { detail: "home" }));
        },
      },
      { icon: "play", label: "Open", action: () => launchApp(app) },
      { icon: "info", label: "Details", action: () => showAppInfo(app) },
      {
        icon: hidden ? "eye" : "eyeSlash",
        label: hidden ? "Show on Home" : "Hide",
        action: () => {
          const hiddenApps = hidden
            ? settings.hiddenApps.filter((id) => id !== app.id)
            : [...settings.hiddenApps, app.id];
          actions.patchSettings({ hiddenApps, showHidden: hidden ? settings.showHidden : true });
          toast(hidden ? `${app.name} is visible again` : `${app.name} hidden`, "ok");
        },
      },
      {
        icon: "folder",
        label: "Open Folder",
        action: () => {
          const dir = app.desktopFile.replace(/\/[^/]+$/, "");
          void api.openTarget(dir).catch(() => toast("Could not open folder", "error"));
        },
      },
    ],
    `tile-${app.id}`,
  );
}

/** tvOS action sheet for a movie / show poster with JustWatch & streaming links. */
export function openMediaMenu(item: MediaItem, _anchor?: HTMLElement): void {
  const profile = store.state.profile;
  const liked = profile.liked.includes(item.id);
  const src = posterUrl(item);

  showActionSheet(
    {
      title: item.title,
      subtitle: [item.genre, item.year, item.stars].filter(Boolean).join(" · ") || "Movie",
      iconSrc: src,
      iconName: "film",
    },
    [
      {
        icon: "search",
        label: "Where to Watch (JustWatch)",
        primary: true,
        action: () => {
          const jw = STREAMING_SERVICES.find((s) => s.id === "justwatch");
          if (jw) openStreaming(item, jw);
        },
      },
      {
        icon: "play",
        label: "Watch on Netflix",
        action: () => {
          const netflix = STREAMING_SERVICES.find((s) => s.id === "netflix");
          if (netflix) openStreaming(item, netflix);
        },
      },
      {
        icon: "film",
        label: "Watch on Prime Video",
        action: () => {
          const prime = STREAMING_SERVICES.find((s) => s.id === "prime");
          if (prime) openStreaming(item, prime);
        },
      },
      {
        icon: "star",
        label: "More Streaming Apps…",
        action: () => playOnPicker(item, `media-${item.id}`),
      },
      {
        icon: "heart",
        label: liked ? "Remove Favorite" : "Favorite / Liked",
        action: () => {
          mediaFeedback(
            item,
            liked ? "hide" : "like",
            liked ? "Removed from favorites" : "Added to favorites ⭐",
          );
        },
      },
      { icon: "external", label: "IMDb Info", action: () => openMedia(item) },
      {
        icon: "ban",
        label: "Not Interested",
        danger: true,
        action: () => mediaFeedback(item, "hide", "Less content like this will be recommended"),
      },
    ],
    `media-${item.id}`,
  );
}

/** Read-only info sheet listing everything the desktop entry exposes. */
export function showAppInfo(app: AppInfo): void {
  const rows: [string, string][] = [
    ["Name", app.name],
    ["Generic name", app.genericName ?? "—"],
    ["Description", app.comment ?? "—"],
    ["Category", app.group],
    ["Exec", app.exec],
    ["Desktop file", app.desktopFile],
    ["Icon", app.iconPath ?? app.iconName ?? "—"],
    ["Source", app.isFlatpak ? "Flatpak" : app.isSnap ? "Snap" : "Native package"],
    ["Terminal", app.terminal ? "Yes" : "No"],
    ["Window class", app.startupWmClass ?? "—"],
    ["Categories", app.categories.join(", ") || "—"],
    ["Keywords", app.keywords.join(", ") || "—"],
  ];

  openOverlay({
    layer: "info",
    className: "overlay--info",
    returnKey: `tile-${app.id}`,
    build: (root, helpers) => {
      const card = el("div", "dialog");
      card.appendChild(el("h2", undefined, app.name));
      const list = el("div", "info-rows");
      for (const [label, value] of rows) {
        const row = el("div", "info-row");
        row.appendChild(el("span", "info-row__label", label));
        row.appendChild(el("span", "info-row__value", value));
        list.appendChild(row);
      }
      card.appendChild(list);
      const actions2 = el("div", "dialog__actions");
      const closeButton = el("button", "btn btn--primary", "Done");
      makeFocusable(
        closeButton,
        {
          onFocus: () => sound.focus(),
          onActivate: () => {
            sound.select();
            helpers.close();
          },
        },
        "info-close",
      );
      actions2.appendChild(closeButton);
      card.appendChild(actions2);
      root.appendChild(card);
      focusEngine.registerZone("info-actions", actions2, 1);
      requestAnimationFrame(() => focusEngine.focusKey("info-close"));
    },
  });
}