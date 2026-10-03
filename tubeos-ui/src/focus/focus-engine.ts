import type { FocusTarget } from "../types";

export type Direction = "up" | "down" | "left" | "right";

interface Handlers {
  onFocus?: () => void;
  onBlur?: () => void;
  onActivate?: () => void;
  onContext?: () => void;
  /** Return true to swallow the arrow key (sliders and text fields use this). */
  onMove?: (direction: Direction) => boolean;
  disabled?: () => boolean;
  /** Stable identity so focus survives a re-render. */
  key?: string;
}

interface ZoneInfo {
  id: string;
  layer: string;
  order: number;
  element: HTMLElement;
}

/**
 * Deterministic, ultra-fast 2D spatial focus engine for TV interfaces.
 * Provides instant row-to-row navigation, prevents focus jitter/loops,
 * and maintains horizontal alignment across shelves.
 */
export class FocusEngine {
  private zones = new Map<string, ZoneInfo>();
  private handlers = new WeakMap<HTMLElement, Handlers>();
  private layerStack: string[] = ["home"];
  private current: HTMLElement | null = null;
  private memory = new Map<string, string>();
  private onFocusChange: ((el: HTMLElement | null) => void) | null = null;

  private get layer(): string {
    return this.layerStack[this.layerStack.length - 1] ?? "home";
  }

  onchange(fn: (el: HTMLElement | null) => void): void {
    this.onFocusChange = fn;
  }

  attach(el: HTMLElement, handlers: Handlers): void {
    this.handlers.set(el, handlers);
    el.classList.add("focusable");
    el.addEventListener("pointerenter", () => this.focusElement(el));
    el.addEventListener("click", (event) => {
      event.stopPropagation();
      if (this.current !== el) {
        this.focusElement(el);
        return;
      }
      this.activate();
    });
    el.addEventListener("contextmenu", (event) => {
      const handler = this.handlers.get(el);
      if (!handler?.onContext) return;
      event.preventDefault();
      this.focusElement(el);
      handler.onContext();
    });
  }

  handlersFor(el: HTMLElement): Handlers | undefined {
    return this.handlers.get(el);
  }

  registerZone(id: string, element: HTMLElement, order: number, layer = this.layer): void {
    this.zones.set(id, { id, layer, order, element });
  }

  unregisterZone(id: string): void {
    this.zones.delete(id);
  }

  pushLayer(layer: string): void {
    this.layerStack.push(layer);
  }

  popLayer(layer?: string): string {
    if (layer) {
      const index = this.layerStack.lastIndexOf(layer);
      if (index > 0) this.layerStack.splice(index, 1);
    } else if (this.layerStack.length > 1) {
      this.layerStack.pop();
    }
    return this.layer;
  }

  get activeLayer(): string {
    return this.layer;
  }

  private collect(): { el: HTMLElement; zone: ZoneInfo }[] {
    const active = this.layer;
    const zones = [...this.zones.values()]
      .filter((zone) => zone.layer === active)
      .sort((a, b) => a.order - b.order);
    const out: { el: HTMLElement; zone: ZoneInfo }[] = [];
    for (const zone of zones) {
      if (!zone.element.isConnected) continue;
      for (const node of zone.element.querySelectorAll<HTMLElement>(".focusable")) {
        if (!node.isConnected) continue;
        const handler = this.handlers.get(node);
        if (handler?.disabled?.()) continue;
        if (node.hasAttribute("data-focus-skip")) continue;
        if (!this.isRendered(node)) continue;
        out.push({ el: node, zone });
      }
    }
    return out;
  }

  private isRendered(el: HTMLElement): boolean {
    if (!el.isConnected) return false;
    const rect = el.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  }

  rebuild(preferKey?: string): void {
    const items = this.collect();
    if (!items.length) {
      this.setCurrent(null);
      return;
    }
    const key = preferKey ?? this.currentKey();
    if (key) {
      const match = items.find((item) => this.handlers.get(item.el)?.key === key);
      if (match) {
        this.setCurrent(match.el);
        return;
      }
    }
    if (this.current && items.some((item) => item.el === this.current)) {
      this.setCurrent(this.current);
      return;
    }
    const remembered = this.bestRemembered(items);
    this.setCurrent(remembered?.el ?? items[0].el);
  }

  private currentKey(): string | undefined {
    return this.current ? this.handlers.get(this.current)?.key : undefined;
  }

  private bestRemembered(items: { el: HTMLElement }[]): { el: HTMLElement } | null {
    for (const key of this.memory.values()) {
      const match = items.find((item) => this.handlers.get(item.el)?.key === key);
      if (match) return match;
    }
    return null;
  }

  private setCurrent(el: HTMLElement | null): void {
    if (this.current && this.current !== el) {
      this.current.classList.remove("is-focused");
      this.handlers.get(this.current)?.onBlur?.();
    }
    this.current = el;
    if (!el) {
      this.onFocusChange?.(null);
      return;
    }
    el.classList.add("is-focused");
    const handler = this.handlers.get(el);
    const zone = this.zoneOf(el);
    if (handler?.key && zone) this.memory.set(zone.id, handler.key);
    handler?.onFocus?.();
    this.ensureVisible(el);
    this.onFocusChange?.(el);
  }

  private zoneOf(el: HTMLElement): { id: string; info: ZoneInfo } | null {
    for (const [id, info] of this.zones) {
      if (info.element.contains(el)) return { id, info };
    }
    return null;
  }

  get focused(): HTMLElement | null {
    return this.current;
  }

  focusElement(el: HTMLElement | null): void {
    if (!el || el === this.current) return;
    if (this.handlers.get(el)?.disabled?.()) return;
    for (const info of this.zones.values()) {
      if (info.layer !== this.layer) continue;
      if (info.element.contains(el)) {
        this.setCurrent(el);
        return;
      }
    }
  }

  focusKey(key: string): boolean {
    const match = this.collect().find((item) => this.handlers.get(item.el)?.key === key);
    if (!match) return false;
    this.setCurrent(match.el);
    return true;
  }

  focusFirst(zoneId?: string): void {
    const items = this.collect().filter((item) => !zoneId || this.zoneOf(item.el)?.id === zoneId);
    if (items.length) this.setCurrent(items[0].el);
  }

  activate(): boolean {
    const handler = this.current ? this.handlers.get(this.current) : undefined;
    if (!handler?.onActivate) return false;
    handler.onActivate();
    return true;
  }

  context(): boolean {
    const handler = this.current ? this.handlers.get(this.current) : undefined;
    if (!handler?.onContext) return false;
    handler.onContext();
    return true;
  }

  /** Move focus deterministically across shelves and rows */
  move(direction: Direction): boolean {
    if (!this.current) {
      this.focusFirst();
      return true;
    }
    const handler = this.handlers.get(this.current);
    if (handler?.onMove?.(direction)) return true;

    // ── 1. Topbar Navigation ────────────────────────────────────────────────
    const currentZone = this.zoneOf(this.current);
    const inTopbar = currentZone?.id === "topbar";

    const allZones = [...this.zones.values()]
      .filter((z) => z.layer === this.layer && z.element.isConnected)
      .sort((a, b) => a.order - b.order);

    const shelfZones = allZones.filter((z) => z.id.startsWith("shelf-"));

    if (inTopbar) {
      if (direction === "left" || direction === "right") {
        return this.moveWithinZone(currentZone.info, direction === "right" ? 1 : -1);
      }
      if (direction === "down") {
        // Jump from Topbar directly into the first shelf
        if (shelfZones.length > 0) {
          const firstShelf = shelfZones[0];
          return this.focusBestInZone(firstShelf, this.current);
        }
        return false;
      }
      return false; // Up on topbar does nothing
    }

    // ── 1.5. Topshelf / Hero Actions Navigation ──────────────────────────────
    if (currentZone && currentZone.id === "topshelf") {
      if (direction === "left" || direction === "right") {
        return this.moveWithinZone(currentZone.info, direction === "right" ? 1 : -1);
      }
      if (direction === "up") {
        const topbarZone = allZones.find((z) => z.id === "topbar");
        if (topbarZone) {
          const activeTab =
            topbarZone.element.querySelector<HTMLElement>(".topbar-item.is-active") ||
            topbarZone.element.querySelector<HTMLElement>(".focusable");
          if (activeTab) {
            this.setCurrent(activeTab);
            return true;
          }
        }
        return false;
      }
      if (direction === "down") {
        if (shelfZones.length > 0) {
          const firstShelf = shelfZones[0];
          return this.focusBestInZone(firstShelf, this.current);
        }
        return false;
      }
    }

    // ── 2. Shelf / Row Navigation in Home / Stage ───────────────────────────
    if (currentZone && currentZone.id.startsWith("shelf-")) {
      const shelfIndex = shelfZones.findIndex((z) => z.id === currentZone.id);

      if (direction === "left" || direction === "right") {
        return this.moveWithinZone(currentZone.info, direction === "right" ? 1 : -1);
      }

      if (direction === "down") {
        if (shelfIndex >= 0 && shelfIndex < shelfZones.length - 1) {
          const nextShelf = shelfZones[shelfIndex + 1];
          return this.focusBestInZone(nextShelf, this.current);
        }
        return false; // At bottom shelf: do not bounce back!
      }

      if (direction === "up") {
        if (shelfIndex > 0) {
          const prevShelf = shelfZones[shelfIndex - 1];
          return this.focusBestInZone(prevShelf, this.current);
        }
        // If at top shelf 0, jump directly to active tab in Topbar
        const topbarZone = allZones.find((z) => z.id === "topbar");
        if (topbarZone) {
          const activeTab = topbarZone.element.querySelector<HTMLElement>(".topbar-item.is-active") ||
                            topbarZone.element.querySelector<HTMLElement>(".focusable");
          if (activeTab) {
            this.setCurrent(activeTab);
            return true;
          }
        }
        return false;
      }
    }

    // ── 3. Overlays / Modals / Generic 2D Spatial Fallback ───────────────────
    return this.moveGeometric(direction);
  }

  private moveWithinZone(zone: ZoneInfo, step: 1 | -1): boolean {
    const focusables = [...zone.element.querySelectorAll<HTMLElement>(".focusable")].filter(
      (el) => this.isRendered(el) && !this.handlers.get(el)?.disabled?.(),
    );
    if (focusables.length <= 1) return false;
    const currentIndex = focusables.findIndex((el) => el === this.current);
    if (currentIndex < 0) return false;

    const nextIndex = currentIndex + step;
    if (nextIndex >= 0 && nextIndex < focusables.length) {
      this.setCurrent(focusables[nextIndex]);
      return true;
    }
    return false;
  }

  private focusBestInZone(targetZone: ZoneInfo, fromEl: HTMLElement): boolean {
    const focusables = [...targetZone.element.querySelectorAll<HTMLElement>(".focusable")].filter(
      (el) => this.isRendered(el) && !this.handlers.get(el)?.disabled?.(),
    );
    if (!focusables.length) return false;

    const fromRect = fromEl.getBoundingClientRect();
    const fromCenterX = fromRect.left + fromRect.width / 2;

    let bestEl: HTMLElement = focusables[0];
    let minDiff = Infinity;

    for (const node of focusables) {
      const rect = node.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const diff = Math.abs(cx - fromCenterX);
      if (diff < minDiff) {
        minDiff = diff;
        bestEl = node;
      }
    }

    this.setCurrent(bestEl);
    return true;
  }

  private moveGeometric(direction: Direction): boolean {
    const items = this.collect();
    if (items.length <= 1 || !this.current) return false;

    const source = this.current.getBoundingClientRect();
    const origin = { x: source.left + source.width / 2, y: source.top + source.height / 2 };

    const isVert = direction === "up" || direction === "down";
    const isDown = direction === "down";
    const isRight = direction === "right";

    const candidates = items.filter((item) => {
      if (item.el === this.current) return false;
      const rect = item.el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) return false;
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;

      if (isVert) {
        return isDown ? cy > origin.y + 4 : cy < origin.y - 4;
      } else {
        return isRight ? cx > origin.x + 4 : cx < origin.x - 4;
      }
    });

    if (!candidates.length) return false;

    let bestEl: HTMLElement | null = null;
    let bestScore = Infinity;

    for (const item of candidates) {
      const rect = item.el.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const deltaX = Math.abs(cx - origin.x);
      const deltaY = Math.abs(cy - origin.y);

      const primary = isVert ? deltaY : deltaX;
      const secondary = isVert ? deltaX : deltaY;
      const score = primary * 1.0 + secondary * 0.4;

      if (score < bestScore) {
        bestScore = score;
        bestEl = item.el;
      }
    }

    if (bestEl) {
      this.setCurrent(bestEl);
      return true;
    }
    return false;
  }

  /** Scroll so the focused tile is centred in its shelf and its row in view. */
  ensureVisible(el: HTMLElement): void {
    const shelf = el.closest<HTMLElement>(".shelf");
    if (shelf) {
      const rect = el.getBoundingClientRect();
      const shelfRect = shelf.getBoundingClientRect();
      const centered =
        shelf.scrollLeft + (rect.left - shelfRect.left) - (shelfRect.width - rect.width) / 2;
      shelf.scrollTo({ left: Math.max(0, centered), behavior: "smooth" });
    }

    const stage = document.getElementById("stage");
    const row = el.closest<HTMLElement>("[data-row]");
    if (stage && row) {
      const rowTop = row.offsetTop;
      const rowBottom = rowTop + row.offsetHeight;
      const viewTop = stage.scrollTop;
      const viewBottom = viewTop + stage.clientHeight;
      const fullyVisible = rowTop >= viewTop - 10 && rowBottom <= viewBottom + 10;
      if (!fullyVisible) {
        const targetScroll = Math.max(0, rowTop - 120);
        stage.scrollTo({ top: targetScroll, behavior: "auto" });
      }
    }

    // Scroll any scrollable container (e.g. .settings-content, dialogs, overlays)
    let parent = el.parentElement;
    while (parent && parent !== document.body && parent !== document.documentElement) {
      const style = window.getComputedStyle(parent);
      if (style.overflowY === "auto" || style.overflowY === "scroll") {
        const elRect = el.getBoundingClientRect();
        const pRect = parent.getBoundingClientRect();
        if (elRect.bottom > pRect.bottom - 24) {
          parent.scrollBy({ top: elRect.bottom - pRect.bottom + 50, behavior: "smooth" });
        } else if (elRect.top < pRect.top + 24) {
          parent.scrollBy({ top: elRect.top - pRect.top - 50, behavior: "smooth" });
        }
        break;
      }
      parent = parent.parentElement;
    }
  }

  get count(): number {
    return this.collect().length;
  }

  get items(): FocusTarget[] {
    return this.collect().map((item) => ({
      el: item.el,
      zone: this.zoneOf(item.el)?.id ?? "",
      key: this.handlers.get(item.el)?.key,
    }));
  }
}

export const focusEngine = new FocusEngine();

export function makeFocusable(
  el: HTMLElement,
  handlers: Handlers,
  focusKey?: string,
): HTMLElement {
  if (focusKey) el.dataset.focusKey = focusKey;
  focusEngine.attach(el, { ...handlers, key: focusKey ?? handlers.key });
  return el;
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