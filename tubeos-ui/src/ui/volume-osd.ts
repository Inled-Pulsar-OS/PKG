import { api } from "../api";
import { sound } from "../sound";
import { store } from "../state";
import { el, icon } from "./icons";

let osdContainer: HTMLElement | null = null;
let osdTimeout: number | null = null;
let currentVolume = 50;
let isMuted = false;

function getVolumeIcon(volume: number, muted: boolean): string {
  if (muted || volume === 0) return "volumeMute";
  if (volume < 35) return "volume";
  return "volume";
}

export function initVolumeOsd(): void {
  // Mount container in DOM
  if (!osdContainer) {
    osdContainer = el("div", "volume-osd");
    document.body.appendChild(osdContainer);
  }

  // Fetch initial audio state
  void api
    .getAudio()
    .then(([vol, muted]) => {
      if (vol !== null) currentVolume = vol;
      if (muted !== null) isMuted = muted;
      store.set({ audio: { volume: currentVolume, muted: isMuted } });
    })
    .catch(() => undefined);

  // Wire hardware multimedia volume keys
  window.addEventListener("keydown", (event) => {
    const key = event.key;
    if (key === "AudioVolumeUp" || key === "VolumeUp" || key === "XF86AudioRaiseVolume") {
      event.preventDefault();
      nudgeVolume(1);
    } else if (key === "AudioVolumeDown" || key === "VolumeDown" || key === "XF86AudioLowerVolume") {
      event.preventDefault();
      nudgeVolume(-1);
    } else if (key === "AudioVolumeMute" || key === "VolumeMute" || key === "XF86AudioMute") {
      event.preventDefault();
      toggleSystemMute();
    }
  });
}

export function showVolumeOsd(vol?: number, muted?: boolean): void {
  if (!osdContainer) initVolumeOsd();
  if (!osdContainer) return;

  const volume = vol !== undefined ? vol : (store.state.audio.volume ?? currentVolume);
  const mute = muted !== undefined ? muted : (store.state.audio.muted ?? isMuted);
  const clamped = Math.max(0, Math.min(100, Math.round(volume)));

  osdContainer.replaceChildren();

  const iconName = getVolumeIcon(clamped, mute);
  const iconWrap = el("div", "volume-osd__icon");
  iconWrap.appendChild(icon(iconName, 22));
  osdContainer.appendChild(iconWrap);

  const barTrack = el("div", "volume-osd__track");
  const barFill = el("div", "volume-osd__fill");
  barFill.style.height = `${mute ? 0 : clamped}%`;
  if (mute) barFill.style.opacity = "0.3";
  barTrack.appendChild(barFill);
  osdContainer.appendChild(barTrack);

  const label = el("div", "volume-osd__label", mute ? "Mute" : `${clamped}%`);
  osdContainer.appendChild(label);

  osdContainer.classList.add("is-visible");

  if (osdTimeout !== null) {
    window.clearTimeout(osdTimeout);
  }
  osdTimeout = window.setTimeout(() => {
    osdContainer?.classList.remove("is-visible");
    osdTimeout = null;
  }, 2000);
}

export function nudgeVolume(direction: 1 | -1): void {
  sound.focus();
  void api
    .audioCommand(direction === 1 ? "volume-up" : "volume-down")
    .then(() => api.getAudio())
    .then(([vol, muted]) => {
      if (vol !== null) currentVolume = vol;
      if (muted !== null) isMuted = muted;
      store.set({ audio: { volume: currentVolume, muted: isMuted } });
      showVolumeOsd(currentVolume, isMuted);
    })
    .catch(() => {
      // Fallback in-memory
      currentVolume = Math.max(0, Math.min(100, currentVolume + direction * 5));
      store.set({ audio: { volume: currentVolume, muted: isMuted } });
      showVolumeOsd(currentVolume, isMuted);
    });
}

export function toggleSystemMute(): void {
  sound.toggle();
  void api
    .audioCommand("volume-mute")
    .then(() => api.getAudio())
    .then(([vol, muted]) => {
      if (vol !== null) currentVolume = vol;
      if (muted !== null) isMuted = muted;
      store.set({ audio: { volume: currentVolume, muted: isMuted } });
      showVolumeOsd(currentVolume, isMuted);
    })
    .catch(() => {
      isMuted = !isMuted;
      store.set({ audio: { volume: currentVolume, muted: isMuted } });
      showVolumeOsd(currentVolume, isMuted);
    });
}
