import { iconUrl, posterUrl } from "../api";
import { focusEngine, makeFocusable } from "../focus/focus-engine";
import { washFor } from "../palette";
import { sound } from "../sound";
import { actions, store } from "../state";
import type { AppInfo, MediaItem } from "../types";
import { el, icon } from "./icons";
import { launchApp, mediaFeedback, openAppMenu, openMedia } from "./tiles";
import { toast } from "./overlay";
import { isIptvFavorite, openTvPlayer, toggleIptvFavorite, type IptvChannel } from "./iptv";

interface HeroState {
  mode: "featured" | "app" | "media" | "iptv";
  app?: AppInfo;
  item?: MediaItem;
  channel?: IptvChannel;
  index: number;
  pinned: boolean;
  returnFocusKey?: string;
}

const state: HeroState = { mode: "featured", index: 0, pinned: false };
let rotateTimer: number | null = null;

function featuredItems(): MediaItem[] {
  const { settings, recommendations } = store.state;
  return recommendations.slice(0, Math.max(6, settings.mediaShelfSize));
}

function backdrop(url: string | null): void {
  const layer = document.getElementById("backdrop-image");
  if (!layer) return;
  if (url) {
    layer.style.backgroundImage = `url("${url}")`;
    layer.classList.add("is-visible");
  } else {
    layer.classList.remove("is-visible");
  }
}

function topGenreLabel(): string {
  const entries = Object.entries(store.state.profile.genres).sort((a, b) => b[1] - a[1]);
  return entries.length ? `mostly ${entries[0][0]}` : "your taste profile";
}

function fallbackPoster(item: MediaItem): HTMLElement {
  const node = el("div", "poster-fallback");
  const wash = washFor(item.genre || item.title);
  node.style.setProperty("--wash-a", wash.a);
  node.style.setProperty("--wash-b", wash.b);
  node.appendChild(el("span", undefined, item.title));
  return node;
}

function pill(
  label: string,
  iconName: string | null,
  variant: "primary" | "normal" | "ghost",
  key: string,
  action: () => void,
): HTMLElement {
  const button = el(
    "button",
    `btn${variant === "primary" ? " btn--primary" : variant === "ghost" ? " btn--ghost" : ""}`,
  );
  if (iconName) button.appendChild(icon(iconName, 16));
  button.appendChild(el("span", undefined, label));
  makeFocusable(
    button,
    {
      onFocus: () => {
        sound.focus();
        state.pinned = true;
      },
      onBlur: () => {
        window.setTimeout(() => {
          if (!focusEngine.focused?.closest("#top-shelf")) state.pinned = false;
        }, 0);
      },
      onActivate: () => {
        sound.select();
        action();
      },
      onMove: (dir) => {
        if (dir === "down") {
          // Return focus directly back to the shelf tile below
          if (state.returnFocusKey && focusEngine.focusKey(state.returnFocusKey)) {
            return true;
          }
        }
        return false;
      },
    },
    key,
  );
  return button;
}

function metaLine(item: MediaItem): HTMLElement {
  const meta = el("div", "hero-meta");
  meta.appendChild(el("span", "hero-badge", item.kind));
  if (item.year) meta.appendChild(el("span", undefined, item.year));
  if (item.stars) {
    meta.appendChild(el("span", "dot"));
    meta.appendChild(el("span", undefined, item.stars));
  }
  return meta;
}

function renderAppHero(container: HTMLElement, app: AppInfo): void {
  const hero = el("div", "hero");
  const art = el("div", "hero-art hero-art--app");
  const src = iconUrl(app.iconName, app.iconPath);
  if (src) {
    const img = el("img");
    img.src = src;
    img.alt = app.name;
    art.appendChild(img);
  } else {
    art.appendChild(el("div", "tile-art__fallback", app.name.slice(0, 2).toUpperCase()));
  }
  hero.appendChild(art);

  const info = el("div", "hero-info");
  info.appendChild(el("div", "hero-eyebrow", app.group || "Application"));
  info.appendChild(el("h1", "hero-title", app.name));

  const meta = el("div", "hero-meta");
  meta.appendChild(
    el("span", "hero-badge", app.isFlatpak ? "Flatpak" : app.isSnap ? "Snap" : "Native"),
  );
  if (app.terminal) meta.appendChild(el("span", "hero-badge", "Terminal"));
  if (app.genericName) meta.appendChild(el("span", undefined, app.genericName));
  const usage = store.state.usage.apps[app.id];
  if (usage && usage.count > 0) {
    meta.appendChild(el("span", "dot"));
    meta.appendChild(el("span", undefined, `Opened ${usage.count}×`));
  }
  info.appendChild(meta);
  if (app.comment) info.appendChild(el("p", "hero-blurb", app.comment));

  const favorite = store.state.settings.favorites.includes(app.id);
  const actionsRow = el("div", "hero-actions");
  actionsRow.appendChild(
    pill(favorite ? "Quitar Favorito" : "Añadir a Favoritos", "star", "primary", "hero-favorite", () => {
      const favorites = favorite
        ? store.state.settings.favorites.filter((id) => id !== app.id)
        : [...store.state.settings.favorites, app.id];
      actions.patchSettings({ favorites });
      toast(
        favorite ? `${app.name} quitado de favoritos` : `${app.name} añadido a favoritos ⭐`,
        "ok",
      );
      renderHero();
    }),
  );
  actionsRow.appendChild(pill("Abrir", "play", "normal", "hero-open", () => launchApp(app)));
  actionsRow.appendChild(pill("Opciones", "info", "ghost", "hero-more", () => openAppMenu(app, actionsRow)));
  info.appendChild(actionsRow);
  hero.appendChild(info);
  container.appendChild(hero);

  const wash = washFor(app.id);
  document.body.style.setProperty("--wash-a", wash.a);
  document.body.style.setProperty("--wash-b", wash.b);
  backdrop(src);
  focusEngine.registerZone("topshelf", actionsRow, 0.5);
}

function renderIptvHero(container: HTMLElement, channel: IptvChannel): void {
  const hero = el("div", "hero");
  const art = el("div", "hero-art iptv-hero-art");
  if (channel.logo) {
    const img = el("img");
    img.src = channel.logo;
    img.alt = channel.name;
    img.onerror = () => {
      img.remove();
      art.appendChild(el("div", "iptv-fallback", channel.name));
    };
    art.appendChild(img);
  } else {
    art.appendChild(el("div", "iptv-fallback", channel.name));
  }
  hero.appendChild(art);

  const info = el("div", "hero-info");
  info.appendChild(el("div", "hero-eyebrow", `${channel.group} · Live TV`));
  info.appendChild(el("h1", "hero-title", channel.name));

  const meta = el("div", "hero-meta");
  meta.appendChild(el("span", "hero-badge", "En Directo"));
  if (isIptvFavorite(channel.id)) {
    meta.appendChild(el("span", "hero-badge", "★ Favorito"));
  }
  info.appendChild(meta);
  info.appendChild(el("p", "hero-blurb", "Emisión en directo a través de IPTV. Pulsa Abrir para ver a pantalla completa."));

  const isFav = isIptvFavorite(channel.id);
  const actionsRow = el("div", "hero-actions");
  actionsRow.appendChild(
    pill(isFav ? "Quitar Favorito" : "Añadir a Favoritos", "star", "primary", "hero-favorite", () => {
      toggleIptvFavorite(channel.id);
      renderHero();
    }),
  );
  actionsRow.appendChild(
    pill("Ver Canal", "play", "normal", "hero-open", () => {
      openTvPlayer([channel], 0);
    }),
  );
  info.appendChild(actionsRow);
  hero.appendChild(info);
  container.appendChild(hero);

  backdrop(null);
  focusEngine.registerZone("topshelf", actionsRow, 0.5);
}

function renderFeatured(container: HTMLElement, items: MediaItem[], index: number): void {
  const item = items[index % items.length];
  if (!item) {
    renderBrandHero(container);
    return;
  }
  const hero = el("div", "hero");
  const art = el("div", "hero-art");
  const src = posterUrl(item);
  if (src) {
    const img = el("img");
    img.src = src;
    img.alt = item.title;
    img.addEventListener("error", () => {
      img.remove();
      art.appendChild(fallbackPoster(item));
    });
    art.appendChild(img);
  } else {
    art.appendChild(fallbackPoster(item));
  }
  hero.appendChild(art);

  const info = el("div", "hero-info");
  info.appendChild(el("div", "hero-eyebrow", "Destacado · Recomendado"));
  info.appendChild(el("h1", "hero-title", item.title));
  info.appendChild(metaLine(item));
  const taste = topGenreLabel();
  info.appendChild(
    el(
      "p",
      "hero-blurb",
      `Metadatos y carátulas de IMDb. ${
        taste === "your taste profile" ? "" : `Tu perfil prefiere ${taste}. `
      }Pulsa ↓ para explorar las filas.`,
    ),
  );

  const actionsRow = el("div", "hero-actions");
  actionsRow.appendChild(pill("Ver en IMDb", "external", "primary", "hero-open", () => openMedia(item)));
  actionsRow.appendChild(
    pill("Recomendar similares", "heart", "normal", "hero-like", () =>
      mediaFeedback(item, "like", "Verás más contenido similar"),
    ),
  );
  info.appendChild(actionsRow);
  hero.appendChild(info);
  container.appendChild(hero);

  const wash = washFor(item.genre || item.title);
  document.body.style.setProperty("--wash-a", wash.a);
  document.body.style.setProperty("--wash-b", wash.b);
  backdrop(src);
  focusEngine.registerZone("topshelf", actionsRow, 0.5);
}

function renderMediaHero(container: HTMLElement, item: MediaItem): void {
  const hero = el("div", "hero");
  const art = el("div", "hero-art");
  const src = posterUrl(item);
  if (src) {
    const img = el("img");
    img.src = src;
    img.alt = item.title;
    img.addEventListener("error", () => {
      img.remove();
      art.appendChild(fallbackPoster(item));
    });
    art.appendChild(img);
  } else {
    art.appendChild(fallbackPoster(item));
  }
  hero.appendChild(art);

  const profile = store.state.profile;
  const liked = profile.liked.includes(item.id);

  const info = el("div", "hero-info");
  info.appendChild(el("div", "hero-eyebrow", item.genre ? `${item.genre} · IMDb` : "IMDb"));
  info.appendChild(el("h1", "hero-title", item.title));
  const meta = metaLine(item);
  if (liked) meta.appendChild(el("span", "hero-badge", "★ Favorito"));
  info.appendChild(meta);
  info.appendChild(
    el(
      "p",
      "hero-blurb",
      `Carátula y sinopsis de IMDb clasificada por el motor de recomendaciones.`,
    ),
  );

  const actionsRow = el("div", "hero-actions");
  actionsRow.appendChild(
    pill(liked ? "Quitar Favorito" : "Añadir a Favoritos", "heart", "primary", "hero-favorite", () => {
      mediaFeedback(
        item,
        liked ? "hide" : "like",
        liked ? "Eliminado de favoritos" : "Añadido a favoritos ⭐",
      );
      renderHero();
    }),
  );
  actionsRow.appendChild(pill("Abrir en IMDb", "external", "normal", "hero-open", () => openMedia(item)));
  info.appendChild(actionsRow);
  hero.appendChild(info);
  container.appendChild(hero);

  const wash = washFor(item.genre || item.title);
  document.body.style.setProperty("--wash-a", wash.a);
  document.body.style.setProperty("--wash-b", wash.b);
  backdrop(src);
  focusEngine.registerZone("topshelf", actionsRow, 0.5);
}

function renderBrandHero(container: HTMLElement): void {
  const hero = el("div", "hero");
  const art = el("div", "hero-art hero-art--app");
  art.appendChild(el("div", "tile-art__fallback", "tv"));
  hero.appendChild(art);

  const info = el("div", "hero-info");
  info.appendChild(el("div", "hero-eyebrow", "Bienvenido"));
  info.appendChild(el("h1", "hero-title", "Tube OS"));
  const meta = el("div", "hero-meta");
  meta.appendChild(el("span", "hero-badge", `${store.state.apps.length} apps`));
  info.appendChild(meta);
  info.appendChild(
    el(
      "p",
      "hero-blurb",
      "Interfaz rápida para tu TV y portátil. Navega con las flechas o mando.",
    ),
  );

  const actionsRow = el("div", "hero-actions");
  actionsRow.appendChild(
    pill("Ajustes", "gear", "primary", "hero-settings", () => {
      document.dispatchEvent(new CustomEvent("launcher:open-settings"));
    }),
  );
  info.appendChild(actionsRow);
  hero.appendChild(info);
  container.appendChild(hero);
  backdrop(null);
  focusEngine.registerZone("topshelf", actionsRow, 0.5);
}

/** Full re-render of the Top Shelf for the current state. */
export function renderHero(): void {
  const container = document.getElementById("top-shelf");
  if (!container) return;
  if (!store.state.settings.showTopShelf) {
    container.replaceChildren();
    return;
  }
  const prevFocusedKey = focusEngine.focused?.closest("#top-shelf")
    ? (focusEngine.focused as HTMLElement).dataset.focusKey
    : undefined;

  const next = el("div", "topshelf-inner");
  if (state.mode === "app" && state.app) {
    renderAppHero(next, state.app);
  } else if (state.mode === "iptv" && state.channel) {
    renderIptvHero(next, state.channel);
  } else if (state.mode === "media" && state.item) {
    renderMediaHero(next, state.item);
  } else {
    const items = featuredItems();
    if (items.length) renderFeatured(next, items, state.index);
    else renderBrandHero(next);
  }
  container.replaceChildren(...next.childNodes);

  if (prevFocusedKey) {
    requestAnimationFrame(() => {
      focusEngine.focusKey(prevFocusedKey);
    });
  }
}

/** Long-press trigger: focus the upper hero actions directly */
export function focusHeroActions(fromElement: HTMLElement): void {
  state.pinned = true;
  setHeroForElement(fromElement);
  state.returnFocusKey = fromElement.dataset.focusKey || fromElement.id;
  renderHero();
  sound.select();
  requestAnimationFrame(() => {
    if (!focusEngine.focusKey("hero-favorite")) {
      focusEngine.focusFirst("topshelf");
    }
  });
}

/** Keep the Top Shelf in sync with whatever tile is focused. */
export function setHeroForElement(element: HTMLElement | null): void {
  if (element?.closest("#top-shelf")) {
    state.pinned = true;
    return;
  }
  const appId = element?.dataset.appId;
  if (appId) {
    const app = store.state.apps.find((entry) => entry.id === appId);
    if (app) {
      if (state.mode === "app" && state.app?.id === app.id) return;
      state.mode = "app";
      state.app = app;
      state.item = undefined;
      state.channel = undefined;
      renderHero();
      return;
    }
  }

  const channelId = element?.dataset.channelId;
  if (channelId) {
    import("./iptv").then(({ getCachedChannels }) => {
      const found = getCachedChannels().find((c) => c.id === channelId);
      if (found) {
        state.mode = "iptv";
        state.channel = found;
        state.app = undefined;
        state.item = undefined;
        renderHero();
      }
    });
    return;
  }

  const mediaId = element?.dataset.mediaId;
  if (mediaId) {
    const item = [...store.state.recommendations, ...store.state.catalog].find(
      (entry) => entry.id === mediaId,
    );
    if (item) {
      if (state.mode === "media" && state.item?.id === item.id) return;
      state.mode = "media";
      state.item = item;
      state.app = undefined;
      state.channel = undefined;
      renderHero();
    }
  }
}

/** Force the Top Shelf back to the featured rotation. */
export function resetHeroToFeatured(): void {
  state.mode = "featured";
  state.pinned = false;
  renderHero();
}

/** Restart the automatic Top Shelf carousel (Settings → Top Shelf interval). */
export function startHeroRotation(): void {
  if (rotateTimer !== null) {
    window.clearInterval(rotateTimer);
    rotateTimer = null;
  }
  if (!store.state.settings.showTopShelf) return;
  const seconds = Math.max(3, store.state.settings.topShelfInterval);
  rotateTimer = window.setInterval(() => {
    const items = featuredItems();
    if (state.mode !== "featured" || items.length < 2) return;
    const inHero = focusEngine.focused?.closest("#top-shelf") != null;
    if (!inHero) return;
    state.index = (state.index + 1) % items.length;
    renderHero();
  }, seconds * 1000);
}