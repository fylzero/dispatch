<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import NameLabel from "./NameLabel.vue";
import TerminalPane from "./TerminalPane.vue";
import { colorRgb } from "../colors";
import { openContextMenu } from "../contextMenu";
import { isMac } from "../platform";
import { settings } from "../settings";
import { type PlacedDivider, layout } from "../splits";
import {
  MIN_PANE_HEIGHT,
  MIN_PANE_WIDTH,
  type TerminalTab,
  closePane,
  closeTab,
  dividerDragging,
  dropTargetPane,
  focusPane,
  focusTerminalPane,
  paneColor,
  paneName,
  paneSelection,
  renamePane,
  renamingPane,
  setPaneColor,
  setTabSize,
  splitPaneById,
} from "../tabs";

const props = defineProps<{ tab: TerminalTab; visible: boolean }>();

const root = ref<HTMLElement | null>(null);
const placed = computed(() => layout(props.tab.root));
/*
 * Panes are listed in a fixed order (by id) and positioned absolutely, so
 * when a split reshapes the tree no pane moves to a new parent element, which
 * would restart its shell.
 */
const panes = computed(() =>
  [...placed.value.panes].sort((a, b) => a.id.localeCompare(b.id, undefined, { numeric: true })),
);
const split = computed(() => placed.value.panes.length > 1);
const resizing = ref<"x" | "y" | null>(null);

const pct = (value: number) => `${value * 100}%`;

function paneStyle(rect: { x: number; y: number; w: number; h: number }) {
  return { left: pct(rect.x), top: pct(rect.y), width: pct(rect.w), height: pct(rect.h) };
}

function dividerStyle(divider: PlacedDivider) {
  return divider.axis === "x"
    ? { left: pct(divider.at), top: pct(divider.from), height: pct(divider.span) }
    : { top: pct(divider.at), left: pct(divider.from), width: pct(divider.span) };
}

let stopDrag: (() => void) | undefined;

/**
 * Drags a divider. The two panes (or groups) on either side trade weight, so
 * nothing else in the tab moves, and neither may shrink past the minimum.
 */
function startResize(event: PointerEvent, divider: PlacedDivider) {
  if (event.button !== 0 || !root.value) return;
  event.preventDefault();
  const children = divider.split.children;
  const a = children[divider.index];
  const b = children[divider.index + 1];
  const sum = children.reduce((total, child) => total + child.weight, 0);
  const box = root.value.getBoundingClientRect();
  const size = divider.axis === "x" ? divider.area.w * box.width : divider.area.h * box.height;
  const minPx = divider.axis === "x" ? MIN_PANE_WIDTH : MIN_PANE_HEIGHT;
  const min = Math.min((minPx / size) * sum, (a.weight + b.weight) / 2);
  const start = divider.axis === "x" ? event.clientX : event.clientY;
  const pair = a.weight + b.weight;
  const startA = a.weight;
  resizing.value = divider.axis;
  dividerDragging.value = true;

  const onMove = (move: PointerEvent) => {
    const delta = (((divider.axis === "x" ? move.clientX : move.clientY) - start) / size) * sum;
    const next = Math.min(pair - min, Math.max(min, startA + delta));
    a.weight = next;
    b.weight = pair - next;
  };
  const onUp = () => {
    resizing.value = null;
    dividerDragging.value = false;
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("pointercancel", onUp);
    stopDrag = undefined;
  };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("pointercancel", onUp);
  stopDrag = onUp;
}

const SPLIT_RIGHT_HINT = isMac ? "⌘D" : "Ctrl+Shift+D";
const SPLIT_DOWN_HINT = isMac ? "⌘⇧D" : "Ctrl+Shift+E";
const COPY_HINT = isMac ? "⌘C" : "Ctrl+Shift+C";
const CLOSE_HINT = isMac ? "⌘W" : "Ctrl+Shift+W";

function onPaneContextMenu(event: MouseEvent, paneId: string) {
  /* Taken now, so Copy puts on the clipboard what was selected when the menu opened. */
  const selected = paneSelection(paneId);
  focusPane(props.tab.id, paneId);
  openContextMenu(event, [
    ...(selected
      ? ([
          {
            kind: "item",
            label: "Copy",
            hint: COPY_HINT,
            action: () => void navigator.clipboard.writeText(selected).catch(() => undefined),
          },
          { kind: "separator" },
        ] as const)
      : []),
    { kind: "item", label: paneName(paneId) ? "Rename Pane…" : "Name Pane…", action: () => startRename(paneId) },
    { kind: "colors", label: "Pane Color", value: paneColor(paneId), pick: (color) => setPaneColor(paneId, color) },
    { kind: "separator" },
    { kind: "item", label: "Split Right", hint: SPLIT_RIGHT_HINT, action: () => splitPaneById(paneId, "vertical") },
    { kind: "item", label: "Split Down", hint: SPLIT_DOWN_HINT, action: () => splitPaneById(paneId, "horizontal") },
    { kind: "separator" },
    {
      kind: "item",
      label: split.value ? "Close Pane" : "Close Tab",
      hint: CLOSE_HINT,
      danger: true,
      action: () => (split.value ? closePane(paneId) : closeTab(props.tab.id)),
    },
  ]);
}

/* Naming a pane in place, in the strip at its top: Enter or clicking away keeps it, Escape drops it. */
const renameDraft = ref("");
const renameInput = ref<HTMLInputElement | null>(null);
const setRenameInput = (el: unknown) => {
  renameInput.value = el instanceof HTMLInputElement ? el : null;
};

function startRename(paneId: string) {
  renameDraft.value = paneName(paneId);
  renamingPane.value = paneId;
}

watch(renamingPane, async (id) => {
  if (!id || !panes.value.some((pane) => pane.id === id)) return;
  await nextTick();
  renameInput.value?.focus();
  renameInput.value?.select();
});

function finishRename(save: boolean) {
  const id = renamingPane.value;
  if (!id) return;
  if (save) renamePane(id, renameDraft.value);
  renamingPane.value = null;
  renameDraft.value = "";
  focusTerminalPane(id);
}

function onRenameKeydown(event: KeyboardEvent) {
  if (event.key === "Enter") {
    event.preventDefault();
    finishRename(true);
  } else if (event.key === "Escape") {
    event.preventDefault();
    finishRename(false);
  }
}

function paneTint(paneId: string) {
  const rgb = colorRgb(paneColor(paneId));
  return rgb ? { "--pane-rgb": rgb } : undefined;
}

let observer: ResizeObserver | undefined;

onMounted(() => {
  if (!root.value) return;
  observer = new ResizeObserver(([entry]) => {
    const { width, height } = entry.contentRect;
    if (width && height) setTabSize(props.tab.id, width, height);
  });
  observer.observe(root.value);
});

onBeforeUnmount(() => {
  stopDrag?.();
  observer?.disconnect();
});
</script>

<template>
  <div ref="root" class="split-root" :class="{ split, [`resizing-${resizing}`]: resizing }">
    <div
      v-for="pane in panes"
      :key="pane.id"
      class="split-pane"
      :class="{
        focused: tab.activePane === pane.id,
        'drop-target': dropTargetPane === pane.id,
        tinted: paneColor(pane.id),
      }"
      :data-pane-id="pane.id"
      :style="[paneStyle(pane.rect), paneTint(pane.id)]"
      @pointerdown="focusPane(tab.id, pane.id)"
      @focusin="focusPane(tab.id, pane.id)"
      @contextmenu="onPaneContextMenu($event, pane.id)"
    >
      <div
        v-if="paneName(pane.id) || renamingPane === pane.id"
        class="pane-header"
        :class="[`label-${settings.paneLabelStyle}`, `align-${settings.paneLabelAlign}`, { renaming: renamingPane === pane.id }]"
        @dblclick="startRename(pane.id)"
      >
        <div class="pane-label">
          <input
            v-if="renamingPane === pane.id"
            :ref="setRenameInput"
            v-model="renameDraft"
            class="pane-rename"
            type="text"
            spellcheck="false"
            autocomplete="off"
            placeholder="Pane name"
            aria-label="Pane name"
            @keydown.stop="onRenameKeydown"
            @blur="finishRename(true)"
            @pointerdown.stop
          />
          <span v-else class="pane-name" :title="paneName(pane.id)"><NameLabel :text="paneName(pane.id)" /></span>
        </div>
      </div>
      <TerminalPane :id="pane.id" :active="visible && tab.activePane === pane.id" />
    </div>
    <div
      v-for="divider in placed.dividers"
      :key="divider.key"
      class="split-divider"
      :class="divider.axis === 'x' ? 'col' : 'row'"
      :style="dividerStyle(divider)"
      role="separator"
      :aria-orientation="divider.axis === 'x' ? 'vertical' : 'horizontal'"
      @pointerdown="startResize($event, divider)"
    />
  </div>
</template>
