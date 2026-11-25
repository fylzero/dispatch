import { ref } from "vue";

/** One row of a right-click menu. */
export type MenuEntry =
  | {
      kind: "item";
      label: string;
      hint?: string;
      danger?: boolean;
      disabled?: boolean;
      /** Shows a checkmark after the label. */
      checked?: boolean;
      action: () => void;
      /** A pencil button at the end of the row that edits the item instead of choosing it. */
      edit?: { label: string; action: () => void };
    }
  | { kind: "separator" }
  /** A row of color swatches; `pick(undefined)` clears the color. */
  | { kind: "colors"; label: string; value?: string; pick: (id: string | undefined) => void };

export interface OpenMenu {
  x: number;
  y: number;
  entries: MenuEntry[];
  /** The button that opened the menu, if any: pressing it again closes the menu instead of reopening it. */
  anchor?: HTMLElement;
}

/** The right-click menu that is showing, if any. Only one is open at a time. */
export const contextMenu = ref<OpenMenu | null>(null);

export function openContextMenu(event: MouseEvent, entries: MenuEntry[]) {
  event.preventDefault();
  event.stopPropagation();
  contextMenu.value = { x: event.clientX, y: event.clientY, entries };
}

export function closeContextMenu() {
  contextMenu.value = null;
}

/**
 * Opens a menu as a dropdown under a button, its right edge lined up with the
 * button's (the menu flips left when it would run off the window). Pressing
 * the button while its menu is open closes it.
 */
export function toggleDropdownMenu(anchor: HTMLElement, entries: MenuEntry[]) {
  if (contextMenu.value?.anchor === anchor) {
    closeContextMenu();
    return;
  }
  const rect = anchor.getBoundingClientRect();
  contextMenu.value = { x: rect.right, y: rect.bottom, entries, anchor };
}
