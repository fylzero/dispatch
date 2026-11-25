<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { LABEL_COLORS } from "../colors";
import { type MenuEntry, closeContextMenu, contextMenu } from "../contextMenu";
import { focusTerminalPane, renamingPane, renamingTab } from "../tabs";

const menuEl = ref<HTMLElement | null>(null);
const position = ref({ left: 0, top: 0 });
const EDGE = 6;

/* Keep the menu inside the window, flipping it left or up from the cursor when it would spill over. */
watch(
  contextMenu,
  async (open) => {
    if (!open) return;
    position.value = { left: open.x, top: open.y };
    await nextTick();
    const el = menuEl.value;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    let left = open.x;
    let top = open.y;
    if (left + width > window.innerWidth - EDGE) left = Math.max(EDGE, open.x - width);
    if (top + height > window.innerHeight - EDGE) top = Math.max(EDGE, open.y - height);
    position.value = { left, top };
    el.querySelector<HTMLElement>("button:not(:disabled)")?.focus({ preventScroll: true });
  },
  { flush: "post" },
);

/** Closes the menu and gives the keyboard back to the terminal, unless the choice took it (a rename field). */
function dismiss() {
  closeContextMenu();
  void nextTick(() => {
    const focused = document.activeElement;
    if (!focused || focused === document.body || menuEl.value?.contains(focused)) focusTerminalPane();
  });
}

function choose(entry: MenuEntry & { kind: "item" }, action = entry.action) {
  if (entry.disabled) return;
  closeContextMenu();
  action();
  /*
   * A rename focuses its field a tick later; handing the keyboard back to the
   * terminal then would blur the field and end the rename at once.
   */
  if (renamingTab.value || renamingPane.value) return;
  void nextTick(() => {
    const focused = document.activeElement;
    if (!focused || focused === document.body) focusTerminalPane();
  });
}

function pick(entry: MenuEntry & { kind: "colors" }, id: string | undefined) {
  closeContextMenu();
  entry.pick(id);
  focusTerminalPane();
}

function onPointerDown(event: PointerEvent) {
  const open = contextMenu.value;
  if (!open || menuEl.value?.contains(event.target as Node)) return;
  /* The button that opened a dropdown closes it itself when clicked. */
  if (open.anchor?.contains(event.target as Node)) return;
  dismiss();
}

function onKeydown(event: KeyboardEvent) {
  if (!contextMenu.value) return;
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    dismiss();
    return;
  }
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  event.preventDefault();
  event.stopPropagation();
  const buttons = Array.from(menuEl.value?.querySelectorAll<HTMLElement>(".context-item:not(:disabled)") ?? []);
  if (!buttons.length) return;
  const index = buttons.indexOf(document.activeElement as HTMLElement);
  const step = event.key === "ArrowDown" ? 1 : -1;
  buttons[(index + step + buttons.length) % buttons.length].focus();
}

function onWindowChange() {
  if (contextMenu.value) dismiss();
}

onMounted(() => {
  window.addEventListener("pointerdown", onPointerDown, true);
  window.addEventListener("keydown", onKeydown, true);
  window.addEventListener("blur", onWindowChange);
  window.addEventListener("resize", onWindowChange);
});

onBeforeUnmount(() => {
  window.removeEventListener("pointerdown", onPointerDown, true);
  window.removeEventListener("keydown", onKeydown, true);
  window.removeEventListener("blur", onWindowChange);
  window.removeEventListener("resize", onWindowChange);
});
</script>

<template>
  <div
    v-if="contextMenu"
    ref="menuEl"
    class="context-menu"
    role="menu"
    :style="{ left: `${position.left}px`, top: `${position.top}px` }"
    @contextmenu.prevent
  >
    <template v-for="(entry, index) in contextMenu.entries" :key="index">
      <div v-if="entry.kind === 'separator'" class="context-separator" role="separator" />
      <div v-else-if="entry.kind === 'colors'" class="context-colors" role="group" :aria-label="entry.label">
        <span class="context-colors-label">{{ entry.label }}</span>
        <div class="context-swatches">
          <button
            class="context-swatch none"
            :class="{ selected: !entry.value }"
            type="button"
            title="No color"
            aria-label="No color"
            @click="pick(entry, undefined)"
          />
          <button
            v-for="color in LABEL_COLORS"
            :key="color.id"
            class="context-swatch"
            :class="{ selected: entry.value === color.id }"
            :style="{ '--swatch-rgb': color.rgb }"
            type="button"
            :title="color.label"
            :aria-label="color.label"
            @click="pick(entry, color.id)"
          />
        </div>
      </div>
      <div v-else-if="entry.edit" class="context-row">
        <button
          class="context-item"
          :class="{ danger: entry.danger }"
          type="button"
          :role="entry.checked === undefined ? 'menuitem' : 'menuitemradio'"
          :aria-checked="entry.checked === undefined ? undefined : entry.checked"
          :disabled="entry.disabled"
          @click="choose(entry)"
        >
          <span class="context-label">
            <span class="context-label-text">{{ entry.label }}</span>
            <svg v-if="entry.checked" class="context-check" viewBox="0 0 16 16" aria-label="Selected">
              <path d="m3.5 8.5 3 3 6-7" />
            </svg>
          </span>
          <span v-if="entry.hint" class="context-hint">{{ entry.hint }}</span>
        </button>
        <button
          class="context-row-edit"
          type="button"
          :title="entry.edit.label"
          :aria-label="entry.edit.label"
          @click="choose(entry, entry.edit.action)"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M10.6 2.9a1.4 1.4 0 0 1 2 2L5.4 12.1 2.6 13l.9-2.8 7.1-7.3Z" />
          </svg>
        </button>
      </div>
      <button
        v-else
        class="context-item"
        :class="{ danger: entry.danger }"
        type="button"
        :role="entry.checked === undefined ? 'menuitem' : 'menuitemradio'"
        :aria-checked="entry.checked === undefined ? undefined : entry.checked"
        :disabled="entry.disabled"
        @click="choose(entry)"
      >
        <span class="context-label">
          <span class="context-label-text">{{ entry.label }}</span>
          <svg v-if="entry.checked" class="context-check" viewBox="0 0 16 16" aria-label="Selected">
            <path d="m3.5 8.5 3 3 6-7" />
          </svg>
        </span>
        <span v-if="entry.hint" class="context-hint">{{ entry.hint }}</span>
      </button>
    </template>
  </div>
</template>
