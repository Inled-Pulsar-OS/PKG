import { focusEngine, makeFocusable } from "../focus/focus-engine";
import { sound } from "../sound";
import { el } from "./icons";

export interface VirtualKeyboardOptions {
  container: HTMLElement;
  initialValue?: string;
  placeholder?: string;
  onInput: (value: string) => void;
  onSubmit?: (value: string) => void;
  onClose?: () => void;
}

type LangId = "es" | "en" | "symbols";

const SUGGESTIONS = ["YouTube", "Netflix", "Plex", "Kodi", "Twitch", "Spotify", "Steam", "Browser"];

const LAYOUTS: Record<LangId, { label: string; rows: string[][] }> = {
  es: {
    label: "ES · Español",
    rows: [
      ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
      ["a", "s", "d", "f", "g", "h", "j", "k", "l", "ñ"],
      ["⇧", "z", "x", "c", "v", "b", "n", "m", "⌫"],
      ["?123", "🌐", "Espacio", ",", ".", "⏎"],
    ],
  },
  en: {
    label: "EN · English",
    rows: [
      ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
      ["a", "s", "d", "f", "g", "h", "j", "k", "l"],
      ["⇧", "z", "x", "c", "v", "b", "n", "m", "⌫"],
      ["?123", "🌐", "Space", ",", ".", "⏎"],
    ],
  },
  symbols: {
    label: "123 · Símbolos",
    rows: [
      ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
      ["@", "#", "$", "%", "&", "*", "-", "+", "(", ")"],
      ["ABC", "/", "!", "?", "\"", "'", ":", ";", "⌫"],
      ["🌐", "Espacio", ",", ".", "⏎"],
    ],
  },
};

export class VirtualKeyboard {
  private container: HTMLElement;
  private rootEl: HTMLElement | null = null;
  private textValue: string = "";
  private currentLang: LangId = "es";
  private isShifted: boolean = false;
  private isSymbols: boolean = false;
  private onInputCb: (value: string) => void;
  private onSubmitCb?: (value: string) => void;
  private onCloseCb?: () => void;
  private isVisible: boolean = false;

  constructor(options: VirtualKeyboardOptions) {
    this.container = options.container;
    this.textValue = options.initialValue || "";
    this.onInputCb = options.onInput;
    this.onSubmitCb = options.onSubmit;
    this.onCloseCb = options.onClose;
  }

  get value(): string {
    return this.textValue;
  }

  setValue(val: string): void {
    this.textValue = val;
  }

  render(): HTMLElement {
    if (!this.rootEl) {
      this.rootEl = el("div", "gboard-osk");
    }
    this.rootEl.replaceChildren();

    const activeLayoutKey: LangId = this.isSymbols ? "symbols" : this.currentLang;
    const layout = LAYOUTS[activeLayoutKey];

    // ── 1. Top Suggestion & Action Bar (Row 0) ──────────────────────────────
    const topBar = el("div", "gboard-topbar");
    focusEngine.registerZone("gboard-topbar", topBar, 80);

    // Language pill
    const langPill = el("button", "gboard-chip gboard-chip--lang", layout.label);
    makeFocusable(
      langPill,
      {
        onFocus: () => sound.focus(),
        onActivate: () => {
          sound.select();
          this.toggleLanguage();
        },
        onMove: (dir) => {
          if (dir === "up") {
            // USER REQUIREMENT: Navigate UP past the top row exits keyboard
            this.close();
            return true;
          }
          return false;
        },
      },
      "gboard-chip-lang",
    );
    topBar.appendChild(langPill);

    // Suggestions strip
    const suggStrip = el("div", "gboard-suggestions");
    SUGGESTIONS.slice(0, 4).forEach((sug, idx) => {
      const chip = el("button", "gboard-chip", sug);
      makeFocusable(
        chip,
        {
          onFocus: () => sound.focus(),
          onActivate: () => {
            sound.select();
            this.textValue = sug;
            this.onInputCb(this.textValue);
          },
          onMove: (dir) => {
            if (dir === "up") {
              this.close();
              return true;
            }
            return false;
          },
        },
        `gboard-sug-${idx}`,
      );
      suggStrip.appendChild(chip);
    });
    topBar.appendChild(suggStrip);

    // Hide Keyboard button
    const hideBtn = el("button", "gboard-chip gboard-chip--close", "✕");
    hideBtn.title = "Ocultar Teclado";
    makeFocusable(
      hideBtn,
      {
        onFocus: () => sound.focus(),
        onActivate: () => {
          sound.back();
          this.close();
        },
        onMove: (dir) => {
          if (dir === "up") {
            this.close();
            return true;
          }
          return false;
        },
      },
      "gboard-btn-close",
    );
    topBar.appendChild(hideBtn);

    this.rootEl.appendChild(topBar);

    // ── 2. Keyboard Keys Grid ────────────────────────────────────────────────
    const keysContainer = el("div", "gboard-keys-container");

    layout.rows.forEach((row, rowIdx) => {
      const rowEl = el("div", "gboard-row");
      focusEngine.registerZone(`gboard-row-${rowIdx}`, rowEl, 81 + rowIdx);

      row.forEach((char, colIdx) => {
        const displayChar = this.isShifted && char.length === 1 ? char.toUpperCase() : char;
        let keyClass = "gboard-key";
        if (char === "Espacio" || char === "Space") keyClass += " gboard-key--space";
        else if (["⇧", "⌫", "?123", "ABC", "🌐"].includes(char)) keyClass += " gboard-key--mod";
        else if (char === "⏎") keyClass += " gboard-key--enter";

        const btn = el("button", keyClass, displayChar);
        makeFocusable(
          btn,
          {
            onFocus: () => sound.focus(),
            onActivate: () => {
              sound.select();
              this.handleKeyPress(char);
            },
          },
          `gboard-k-${rowIdx}-${colIdx}`,
        );
        rowEl.appendChild(btn);
      });

      keysContainer.appendChild(rowEl);
    });

    this.rootEl.appendChild(keysContainer);
    return this.rootEl;
  }

  private handleKeyPress(char: string): void {
    if (char === "⇧") {
      this.isShifted = !this.isShifted;
      this.render();
      return;
    }
    if (char === "?123") {
      this.isSymbols = true;
      this.render();
      return;
    }
    if (char === "ABC") {
      this.isSymbols = false;
      this.render();
      return;
    }
    if (char === "🌐") {
      this.toggleLanguage();
      return;
    }
    if (char === "Espacio" || char === "Space") {
      this.textValue += " ";
      this.onInputCb(this.textValue);
      return;
    }
    if (char === "⌫") {
      this.textValue = this.textValue.slice(0, -1);
      this.onInputCb(this.textValue);
      return;
    }
    if (char === "⏎") {
      if (this.onSubmitCb) {
        this.onSubmitCb(this.textValue);
      }
      return;
    }

    const typed = this.isShifted ? char.toUpperCase() : char.toLowerCase();
    this.textValue += typed;
    this.onInputCb(this.textValue);

    if (this.isShifted) {
      this.isShifted = false;
      this.render();
    }
  }

  private toggleLanguage(): void {
    this.currentLang = this.currentLang === "es" ? "en" : "es";
    this.isSymbols = false;
    this.render();
    focusEngine.rebuild();
  }

  show(): void {
    this.isVisible = true;
    const osk = this.render();
    if (!this.container.contains(osk)) {
      this.container.appendChild(osk);
    }
    focusEngine.focusFirst("gboard-row-1");
  }

  close(): void {
    this.isVisible = false;
    if (this.rootEl && this.container.contains(this.rootEl)) {
      this.rootEl.remove();
    }
    for (let i = 0; i < 6; i++) {
      focusEngine.unregisterZone(`gboard-row-${i}`);
    }
    focusEngine.unregisterZone("gboard-topbar");
    this.onCloseCb?.();
  }

  get isOpen(): boolean {
    return this.isVisible;
  }
}
