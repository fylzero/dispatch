import { ref } from "vue";

/** One row of a right-click menu. */
export type MenuEntry =
  | { kind: "item"; label: string; hint?: string; danger?: boolean; disabled?: boolean; action: () => void }
  | { kind: "separator" }
  /** A row of color swatches; `pick(undefined)` clears the color. */
  | { kind: "colors"; label: string; value?: string; pick: (id: string | undefined) => void };

export interface OpenMenu {
  x: number;
  y: number;
  entries: MenuEntry[];
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
