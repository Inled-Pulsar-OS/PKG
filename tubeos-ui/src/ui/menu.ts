import { focusEngine, makeFocusable } from "../focus/focus-engine";
import { sound } from "../sound";
import { el, icon } from "./icons";

export interface MenuOption {
  icon: string;
  label: string;
  action: () => void;
  primary?: boolean;
  danger?: boolean;
}

export interface ActionSheetMeta {
  title: string;
  subtitle?: string;
  iconSrc?: string | null;
  iconName?: string;
}

let activeSheet: { node: HTMLElement; close: () => void } | null = null;

export function isMenuOpen(): boolean {
  return activeSheet !== null;
}

/** Close any open bottom action sheet */
export function closeMenu(): boolean {
  if (!activeSheet) return false;
  activeSheet.close();
  return true;
}

/**
 * TV Remote-friendly Bottom Action Sheet (Footer Drawer).
 * Displays a bottom dock with horizontal action pills.
 * Navigable with Left/Right arrows, Enter to execute, Down or Esc to dismiss.
 */
export function showActionSheet(
  meta: ActionSheetMeta,
  options: MenuOption[],
  returnKey?: string,
): void {
  closeMenu();

  const layer = "action-sheet";
  const backdrop = el("div", "sheet-backdrop");
  const sheet = el("div", "sheet-footer");

  let closed = false;

  const close = (): void => {
    if (closed) return;
    closed = true;
    sheet.classList.remove("is-open");
    backdrop.classList.remove("is-open");
    focusEngine.popLayer(layer);
    focusEngine.unregisterZone("sheet-actions");
    activeSheet = null;

    if (returnKey) {
      focusEngine.focusKey(returnKey);
    } else {
      focusEngine.rebuild();
    }

    window.setTimeout(() => {
      backdrop.remove();
      sheet.remove();
    }, 150);
  };

  // Header / Info section (Left)
  const header = el("div", "sheet-header");
  if (meta.iconSrc) {
    const art = el("div", "sheet-art");
    const img = el("img");
    img.src = meta.iconSrc;
    img.alt = meta.title;
    art.appendChild(img);
    header.appendChild(art);
  } else if (meta.iconName) {
    const art = el("div", "sheet-art");
    art.appendChild(icon(meta.iconName, 26));
    header.appendChild(art);
  }

  const metaText = el("div", "sheet-meta");
  metaText.appendChild(el("h2", "sheet-title", meta.title));
  if (meta.subtitle) {
    metaText.appendChild(el("p", "sheet-sub", meta.subtitle));
  }
  header.appendChild(metaText);
  sheet.appendChild(header);

  // Actions row (Horizontal pills)
  const actionsRow = el("div", "sheet-actions");
  options.forEach((opt, idx) => {
    const btn = el(
      "button",
      `btn${opt.primary ? " btn--primary" : ""}${opt.danger ? " btn--danger" : ""}`,
    );
    btn.appendChild(icon(opt.icon, 16));
    btn.appendChild(el("span", undefined, opt.label));

    makeFocusable(
      btn,
      {
        onFocus: () => sound.focus(),
        onActivate: () => {
          sound.select();
          close();
          opt.action();
        },
        onMove: (dir) => {
          if (dir === "down") {
            sound.back();
            close();
            return true;
          }
          return false;
        },
      },
      `sheet-opt-${idx}`,
    );
    actionsRow.appendChild(btn);
  });
  sheet.appendChild(actionsRow);

  const container = document.getElementById("overlays") ?? document.body;
  container.appendChild(backdrop);
  container.appendChild(sheet);

  focusEngine.pushLayer(layer);
  focusEngine.registerZone("sheet-actions", actionsRow, 1, layer);

  backdrop.addEventListener("pointerdown", () => close());

  requestAnimationFrame(() => {
    backdrop.classList.add("is-open");
    sheet.classList.add("is-open");
    focusEngine.focusKey("sheet-opt-0");
  });

  activeSheet = { node: sheet, close };
}

/** Legacy alias for backwards compatibility */
export function showActionMenu(
  anchor: HTMLElement,
  options: MenuOption[],
  returnKey?: string,
): void {
  const title = anchor.querySelector(".tile-label")?.textContent || "Opciones";
  const sub = anchor.querySelector(".tile-sub")?.textContent;
  const img = anchor.querySelector<HTMLImageElement>("img")?.src;
  showActionSheet({ title, subtitle: sub, iconSrc: img }, options, returnKey);
}