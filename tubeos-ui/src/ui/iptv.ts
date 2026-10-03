import { api } from "../api";
import Hls from "hls.js";
import { attachLongPress, focusEngine, makeFocusable } from "../focus/focus-engine";
import { sound } from "../sound";
import { el, icon } from "./icons";
import { showActionSheet } from "./menu";
import { toast } from "./overlay";

export interface IptvChannel {
  id: string;
  name: string;
  logo: string;
  group: string;
  url: string;
  country: string;
}

export interface IptvSource {
  id: string;
  name: string;
  url: string;
}

const CUSTOM_SOURCES_KEY = "tubeos_custom_iptv_sources";
const CUSTOM_FEED_URL_KEY = "tubeos_custom_iptv_feed_url";
const DEFAULT_COUNTRY_KEY = "tubeos_iptv_country";
const FAVORITES_KEY = "tubeos_iptv_favorites";

let cachedChannels: IptvChannel[] = [];
let currentSourceUrl = "";
let hlsInstance: Hls | null = null;
let activePlayingChannelIndex = -1;

// High-performance in-memory Set for O(1) favorite lookups
let favoritesSet: Set<string> = new Set();
try {
  const raw = localStorage.getItem(FAVORITES_KEY);
  if (raw) favoritesSet = new Set(JSON.parse(raw));
} catch {}

function saveFavorites(): void {
  try {
    localStorage.setItem(FAVORITES_KEY, JSON.stringify([...favoritesSet]));
  } catch {}
}

/** Detect country code from browser locale (e.g. es-ES -> es). */
export function getDetectedCountry(): string {
  const stored = localStorage.getItem(DEFAULT_COUNTRY_KEY);
  if (stored) return stored.toLowerCase();
  const lang = navigator.language || "es";
  const code = lang.includes("-") ? lang.split("-")[1] : lang;
  return (code || "es").toLowerCase();
}

export function setIptvCountry(country: string): void {
  localStorage.setItem(DEFAULT_COUNTRY_KEY, country.toLowerCase());
  cachedChannels = [];
  currentSourceUrl = "";
}

export function getCustomFeedUrl(): string {
  return localStorage.getItem(CUSTOM_FEED_URL_KEY) || "";
}

export function setCustomFeedUrl(url: string): void {
  localStorage.setItem(CUSTOM_FEED_URL_KEY, url.trim());
  cachedChannels = [];
  currentSourceUrl = "";
}

export function getIptvFavorites(): string[] {
  return [...favoritesSet];
}

export function isIptvFavorite(channelId: string): boolean {
  return favoritesSet.has(channelId);
}

export function toggleIptvFavorite(channelId: string): boolean {
  let isFav = false;
  if (favoritesSet.has(channelId)) {
    favoritesSet.delete(channelId);
    isFav = false;
    toast("Removed from favorite channels", "info");
  } else {
    favoritesSet.add(channelId);
    isFav = true;
    toast("Added to favorite channels ⭐", "ok");
  }
  saveFavorites();
  return isFav;
}

export function getCustomSources(): IptvSource[] {
  try {
    const raw = localStorage.getItem(CUSTOM_SOURCES_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

export function saveCustomSource(source: IptvSource): void {
  const current = getCustomSources();
  current.push(source);
  localStorage.setItem(CUSTOM_SOURCES_KEY, JSON.stringify(current));
}

/** Parse M3U playlist and strictly filter channels with valid URLs */
export function parseM3u(text: string, country = ""): IptvChannel[] {
  const lines = text.split("\n");
  const channels: IptvChannel[] = [];
  const seenNames = new Set<string>();
  let currentChannel: Partial<IptvChannel> | null = null;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (line.startsWith("#EXTINF:")) {
      const nameMatch = line.match(/,(.+)$/);
      let name = nameMatch ? nameMatch[1].trim() : `Channel ${channels.length + 1}`;
      // Clean up common quality/resolution annotations for cleaner presentation
      name = name.replace(/\s*\((1080p|720p|576p|480p|SD|HD|FHD)\)/gi, "").trim();

      const logoMatch = line.match(/tvg-logo="([^"]+)"/i);
      const logo = logoMatch ? logoMatch[1] : "";
      const groupMatch = line.match(/group-title="([^"]+)"/i);
      const group = groupMatch ? groupMatch[1] : "General";
      const idMatch = line.match(/tvg-id="([^"]+)"/i);
      const id = idMatch && idMatch[1] ? idMatch[1] : name.toLowerCase().replace(/[^a-z0-9]/g, "-");

      currentChannel = {
        id,
        name,
        logo,
        group,
        country,
      };
    } else if ((line.startsWith("http://") || line.startsWith("https://")) && currentChannel) {
      if (currentChannel.name && currentChannel.name.trim().length > 0 && line.length > 8) {
        const dedupeKey = `${currentChannel.name.toLowerCase()}`;
        if (!seenNames.has(dedupeKey)) {
          seenNames.add(dedupeKey);
          currentChannel.url = line;
          channels.push(currentChannel as IptvChannel);
        }
      }
      currentChannel = null;
    }
  }
  return channels;
}

/** Fetch country channels from Custom Feed, TDTChannels (Spain), or iptv-org */
export async function loadIptvChannels(forceCountry?: string): Promise<IptvChannel[]> {
  const customUrl = getCustomFeedUrl();
  if (customUrl) {
    if (cachedChannels.length > 0 && currentSourceUrl === customUrl) {
      return cachedChannels;
    }
    try {
      const text = await api.fetchText(customUrl);
      const parsed = parseM3u(text, "custom");
      if (parsed.length > 0) {
        cachedChannels = parsed;
        currentSourceUrl = customUrl;
        return cachedChannels;
      }
    } catch (err) {
      console.warn("Custom M3U feed failed to load:", err);
    }
  }

  const country = (forceCountry || getDetectedCountry()).toLowerCase();

  // For Spain, prioritize TDTChannels comprehensive live catalog
  if (country === "es") {
    const tdtUrl = "https://www.tdtchannels.com/lists/tv.m3u";
    if (cachedChannels.length > 0 && currentSourceUrl === tdtUrl) {
      return cachedChannels;
    }
    try {
      const text = await api.fetchText(tdtUrl);
      const parsed = parseM3u(text, "es");
      if (parsed.length > 0) {
        cachedChannels = parsed;
        currentSourceUrl = tdtUrl;
        return cachedChannels;
      }
    } catch (err) {
      console.warn("TDTChannels fetch failed, falling back to iptv-org:", err);
    }
  }

  const url = `https://iptv-org.github.io/iptv/countries/${country}.m3u`;
  if (cachedChannels.length > 0 && currentSourceUrl === url) {
    return cachedChannels;
  }

  try {
    const text = await api.fetchText(url);
    cachedChannels = parseM3u(text, country);
    currentSourceUrl = url;
    return cachedChannels;
  } catch (err) {
    console.warn("Could not fetch country IPTV, falling back to ES:", err);
  }

  // Fallback to Spain if chosen country failed
  if (country !== "es") {
    return loadIptvChannels("es");
  }

  return cachedChannels;
}

export function getCachedChannels(): IptvChannel[] {
  return cachedChannels;
}

/** Bottom Action Sheet for IPTV Channel */
export function openIptvMenu(channel: IptvChannel, channels: IptvChannel[], index: number): void {
  const isFav = isIptvFavorite(channel.id);
  showActionSheet(
    {
      title: channel.name,
      subtitle: `${channel.group} · Live TV`,
      iconSrc: channel.logo || null,
      iconName: "tv",
    },
    [
      {
        icon: "star",
        label: isFav ? "Remove Favorite" : "Add to Favorites",
        primary: true,
        action: () => {
          toggleIptvFavorite(channel.id);
          document.dispatchEvent(new CustomEvent("launcher:tab-preview", { detail: "home" }));
        },
      },
      {
        icon: "play",
        label: "Watch Channel",
        action: () => openTvPlayer(channels, index),
      },
    ],
    `tv-${channel.id}`,
  );
}

/** Fullscreen HLS M3U8 Player with Interactive OSD on Enter */
export function openTvPlayer(channels: IptvChannel[], startIndex: number): void {
  activePlayingChannelIndex = startIndex;
  const channel = channels[startIndex];
  if (!channel) return;

  sound.launch();

  let playerModal = document.getElementById("tv-player-modal");
  if (!playerModal) {
    playerModal = el("div", "tv-player-modal");
    playerModal.id = "tv-player-modal";
    document.body.appendChild(playerModal);
  }

  playerModal.replaceChildren();

  const video = document.createElement("video");
  video.className = "tv-player__video";
  video.autoplay = true;
  video.controls = false;
  video.playsInline = true;
  playerModal.appendChild(video);

  // Top Minimal HUD
  const hud = el("div", "tv-player__hud");
  const channelLogo = el("div", "tv-player__logo");
  if (channel.logo) {
    const img = el("img");
    img.src = channel.logo;
    img.onerror = () => {
      img.remove();
      channelLogo.appendChild(icon("tv", 28));
    };
    channelLogo.appendChild(img);
  } else {
    channelLogo.appendChild(icon("tv", 28));
  }
  hud.appendChild(channelLogo);

  const meta = el("div", "tv-player__meta");
  const chTitle = el("div", "tv-player__title", channel.name);
  const chSub = el("div", "tv-player__sub", `${channel.group} · Live TV`);
  meta.appendChild(chTitle);
  meta.appendChild(chSub);
  hud.appendChild(meta);

  const favBadge = el("div", "tv-player__fav-btn");
  const updateFavBadge = (isFav: boolean) => {
    favBadge.replaceChildren();
    favBadge.appendChild(icon("star", 14));
    favBadge.appendChild(el("span", undefined, isFav ? "Favorite" : "Mark Fav"));
  };
  updateFavBadge(isIptvFavorite(channel.id));
  hud.appendChild(favBadge);

  const controlsHint = el("div", "tv-player__hint");
  const hintItem1 = el("span", "tv-hint-item");
  hintItem1.appendChild(icon("arrowUp", 14));
  hintItem1.appendChild(icon("arrowDown", 14));
  hintItem1.appendChild(el("span", undefined, "Channel"));
  controlsHint.appendChild(hintItem1);

  const hintItem2 = el("span", "tv-hint-item");
  hintItem2.appendChild(icon("play", 14));
  hintItem2.appendChild(el("span", undefined, "Pause"));
  controlsHint.appendChild(hintItem2);

  const hintItem3 = el("span", "tv-hint-item");
  hintItem3.appendChild(icon("gear", 14));
  hintItem3.appendChild(el("span", undefined, "Menu (Enter)"));
  controlsHint.appendChild(hintItem3);

  const hintItem4 = el("span", "tv-hint-item");
  hintItem4.appendChild(icon("close", 14));
  hintItem4.appendChild(el("span", undefined, "Exit (Esc)"));
  controlsHint.appendChild(hintItem4);
  hud.appendChild(controlsHint);
  playerModal.appendChild(hud);

  const spinner = el("div", "tv-player__spinner");
  playerModal.appendChild(spinner);

  // Dedicated Error Banner
  const errorBox = el("div", "tv-player__error");
  errorBox.style.display = "none";
  const errorIcon = el("div", "tv-player__error-icon");
  errorIcon.appendChild(icon("alertCircle", 48));
  errorBox.appendChild(errorIcon);
  errorBox.appendChild(el("div", "tv-player__error-title", "Stream Unavailable"));
  errorBox.appendChild(
    el("div", "tv-player__error-desc", "The channel is currently offline or the stream is unreachable at this moment."),
  );

  const errorHint = el("div", "tv-player__error-hint");
  errorHint.appendChild(icon("arrowUp", 14));
  errorHint.appendChild(icon("arrowDown", 14));
  errorHint.appendChild(el("span", undefined, " Change channel  ·  "));
  errorHint.appendChild(icon("close", 14));
  errorHint.appendChild(el("span", undefined, " Esc to return"));
  errorBox.appendChild(errorHint);
  playerModal.appendChild(errorBox);

  // Interactive OSD Overlay Drawer (opened on Enter)
  const osd = el("div", "tv-player__osd");
  const osdInfo = el("div", "tv-osd-info");
  const osdTitle = el("h2", "tv-osd-title", channel.name);
  const osdSub = el("p", "tv-osd-sub", `${channel.group} · Live Broadcast`);
  const osdInfoText = el("div", "tv-osd-info-text");
  osdInfoText.appendChild(osdTitle);
  osdInfoText.appendChild(osdSub);
  osdInfo.appendChild(osdInfoText);
  osd.appendChild(osdInfo);

  const osdActions = el("div", "tv-osd-actions");

  // OSD Action Pills
  const favPill = el("button", "btn btn--primary");
  favPill.appendChild(icon("star", 16));
  const favPillLabel = el("span", undefined, isIptvFavorite(channel.id) ? "Remove Favorite" : "Add to Favorites");
  favPill.appendChild(favPillLabel);

  const playPill = el("button", "btn");
  playPill.appendChild(icon("pause", 16));
  const playPillLabel = el("span", undefined, "Pause");
  playPill.appendChild(playPillLabel);

  const nextPill = el("button", "btn");
  nextPill.appendChild(icon("arrowRight", 16));
  nextPill.appendChild(el("span", undefined, "Next Channel"));

  const prevPill = el("button", "btn");
  prevPill.appendChild(icon("arrowLeft", 16));
  prevPill.appendChild(el("span", undefined, "Previous Channel"));

  const exitPill = el("button", "btn btn--danger");
  exitPill.appendChild(icon("close", 16));
  exitPill.appendChild(el("span", undefined, "Exit Live TV"));

  osdActions.appendChild(favPill);
  osdActions.appendChild(playPill);
  osdActions.appendChild(nextPill);
  osdActions.appendChild(prevPill);
  osdActions.appendChild(exitPill);
  osd.appendChild(osdActions);
  playerModal.appendChild(osd);

  playerModal.classList.add("is-active");
  focusEngine.pushLayer("tv-player");

  let isOsdOpen = false;
  let hudTimer: number | null = null;

  const showHud = (ms = 3500) => {
    hud.classList.remove("is-hidden");
    if (hudTimer) window.clearTimeout(hudTimer);
    hudTimer = window.setTimeout(() => {
      if (!isOsdOpen) hud.classList.add("is-hidden");
    }, ms);
  };

  const openOsd = () => {
    isOsdOpen = true;
    hud.classList.remove("is-hidden");
    osd.classList.add("is-open");
    focusEngine.pushLayer("tv-osd");
    focusEngine.registerZone("osd-zone", osdActions, 1, "tv-osd");
    requestAnimationFrame(() => {
      focusEngine.focusKey("osd-fav");
    });
  };

  const closeOsd = () => {
    if (!isOsdOpen) return;
    isOsdOpen = false;
    osd.classList.remove("is-open");
    focusEngine.popLayer("tv-osd");
    focusEngine.unregisterZone("osd-zone");
    showHud(2000);
  };

  // Wire OSD Focus Actions
  makeFocusable(
    favPill,
    {
      onFocus: () => sound.focus(),
      onActivate: () => {
        const cur = channels[activePlayingChannelIndex];
        if (cur) {
          const isFav = toggleIptvFavorite(cur.id);
          updateFavBadge(isFav);
          favPillLabel.textContent = isFav ? "Quitar Favorito" : "Añadir Favorito";
        }
      },
      onMove: (dir) => {
        if (dir === "down") {
          closeOsd();
          return true;
        }
        return false;
      },
    },
    "osd-fav",
  );

  makeFocusable(
    playPill,
    {
      onFocus: () => sound.focus(),
      onActivate: () => {
        if (video.paused) {
          video.play();
          playPillLabel.textContent = "Pausar";
        } else {
          video.pause();
          playPillLabel.textContent = "Reanudar";
        }
      },
      onMove: (dir) => {
        if (dir === "down") {
          closeOsd();
          return true;
        }
        return false;
      },
    },
    "osd-play",
  );

  makeFocusable(
    nextPill,
    {
      onFocus: () => sound.focus(),
      onActivate: () => {
        activePlayingChannelIndex = (activePlayingChannelIndex + 1) % channels.length;
        playStream(channels[activePlayingChannelIndex]);
      },
      onMove: (dir) => {
        if (dir === "down") {
          closeOsd();
          return true;
        }
        return false;
      },
    },
    "osd-next",
  );

  makeFocusable(
    prevPill,
    {
      onFocus: () => sound.focus(),
      onActivate: () => {
        activePlayingChannelIndex = (activePlayingChannelIndex - 1 + channels.length) % channels.length;
        playStream(channels[activePlayingChannelIndex]);
      },
      onMove: (dir) => {
        if (dir === "down") {
          closeOsd();
          return true;
        }
        return false;
      },
    },
    "osd-prev",
  );

  makeFocusable(
    exitPill,
    {
      onFocus: () => sound.focus(),
      onActivate: () => closePlayer(),
      onMove: (dir) => {
        if (dir === "down") {
          closeOsd();
          return true;
        }
        return false;
      },
    },
    "osd-exit",
  );

  const playStream = (ch: IptvChannel) => {
    chTitle.textContent = ch.name;
    chSub.textContent = `${ch.group} · Live TV`;
    osdTitle.textContent = ch.name;
    osdSub.textContent = `${ch.group} · Live Broadcast`;
    const isFav = isIptvFavorite(ch.id);
    updateFavBadge(isFav);
    favPillLabel.textContent = isFav ? "Remove Favorite" : "Add to Favorites";
    spinner.style.display = "block";
    errorBox.style.display = "none";
    showHud();

    if (hlsInstance) {
      hlsInstance.destroy();
      hlsInstance = null;
    }

    const onPlaybackError = (errDesc: string) => {
      spinner.style.display = "none";
      errorBox.style.display = "flex";
      console.warn(`IPTV Playback error on ${ch.name}:`, errDesc);
    };

    if (Hls.isSupported() && (ch.url.includes(".m3u8") || !ch.url.match(/\.(mp4|webm|mkv|ogg|mov)($|\?)/i))) {
      hlsInstance = new Hls({ enableWorker: true, lowLatencyMode: true });
      hlsInstance.loadSource(ch.url);
      hlsInstance.attachMedia(video);
      hlsInstance.on(Hls.Events.MANIFEST_PARSED, () => {
        spinner.style.display = "none";
        video.play().catch(() => undefined);
      });
      hlsInstance.on(Hls.Events.ERROR, (_, data) => {
        if (data.fatal) {
          // If Hls.js encountered a fatal error, try native video element playback fallback
          console.warn(`HLS failed for ${ch.name}, falling back to native video element:`, data.details);
          if (hlsInstance) {
            hlsInstance.destroy();
            hlsInstance = null;
          }
          video.src = ch.url;
          video.play().catch(() => onPlaybackError(data.details || "Fatal playback error"));
        }
      });
    } else {
      video.src = ch.url;
      video.onloadeddata = () => {
        spinner.style.display = "none";
      };
      video.onerror = () => {
        onPlaybackError("Stream format not supported or channel offline");
      };
      video.play().catch(() => onPlaybackError("Autoplay rejected"));
    }
  };

  playStream(channel);

  const closePlayer = () => {
    if (hlsInstance) {
      hlsInstance.destroy();
      hlsInstance = null;
    }
    video.pause();
    video.src = "";
    if (isOsdOpen) focusEngine.popLayer("tv-osd");
    focusEngine.popLayer("tv-player");
    playerModal?.classList.remove("is-active");
    playerModal?.remove();
    window.removeEventListener("keydown", playerKeyHandler);
  };

  const playerKeyHandler = (e: KeyboardEvent) => {
    if (e.key === "Escape" || e.key === "Backspace") {
      e.preventDefault();
      if (isOsdOpen) {
        closeOsd();
      } else {
        closePlayer();
      }
      return;
    }

    if (isOsdOpen) {
      // In OSD layer, let focus-engine handle arrow keys and Enter
      return;
    }

    // Direct Remote keys when OSD is closed
    if (e.key === "ArrowUp" || e.key === "ArrowRight") {
      e.preventDefault();
      activePlayingChannelIndex = (activePlayingChannelIndex + 1) % channels.length;
      playStream(channels[activePlayingChannelIndex]);
    } else if (e.key === "ArrowDown" || e.key === "ArrowLeft") {
      e.preventDefault();
      activePlayingChannelIndex = (activePlayingChannelIndex - 1 + channels.length) % channels.length;
      playStream(channels[activePlayingChannelIndex]);
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      openOsd();
    } else if (e.key === "f" || e.key === "F" || e.key === "x" || e.key === "X") {
      e.preventDefault();
      const currentCh = channels[activePlayingChannelIndex];
      if (currentCh) {
        const isFav = toggleIptvFavorite(currentCh.id);
        updateFavBadge(isFav);
        showHud();
      }
    }
  };

  window.addEventListener("keydown", playerKeyHandler);
}

/** Channel Card Component */
export function iptvChannelTile(
  ch: IptvChannel,
  focusKey: string,
  channels: IptvChannel[],
  index: number,
): HTMLElement {
  const tile = el("button", "tile iptv-tile");
  tile.dataset.channelId = ch.id;

  const isFav = isIptvFavorite(ch.id);

  const art = el("div", "tile-art iptv-art");
  if (ch.logo) {
    const img = el("img");
    img.src = ch.logo;
    img.alt = ch.name;
    img.loading = "lazy";
    img.onerror = () => {
      img.remove();
      art.appendChild(el("div", "iptv-fallback", ch.name));
    };
    art.appendChild(img);
  } else {
    art.appendChild(el("div", "iptv-fallback", ch.name));
  }

  const liveBadge = el("div", "iptv-live-badge", "LIVE");
  art.appendChild(liveBadge);

  if (isFav) {
    const favIcon = el("div", "iptv-fav-badge", "★");
    art.appendChild(favIcon);
  }

  tile.appendChild(art);

  const label = el("div", "tile-label", ch.name);
  tile.appendChild(label);

  attachLongPress(tile, () => {
    openIptvMenu(ch, channels, index);
  });

  makeFocusable(
    tile,
    {
      onFocus: () => sound.focus(),
      onActivate: () => openTvPlayer(channels, index),
      onContext: () => openIptvMenu(ch, channels, index),
    },
    focusKey,
  );

  return tile;
}
