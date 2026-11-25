<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import TerminalPane from "./TerminalPane.vue";
import { type PlacedDivider, layout } from "../splits";
import { MIN_PANE_HEIGHT, MIN_PANE_WIDTH, type TerminalTab, dividerDragging, focusPane, setTabSize } from "../tabs";

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
      :class="{ focused: tab.activePane === pane.id }"
      :style="paneStyle(pane.rect)"
      @pointerdown="focusPane(tab.id, pane.id)"
      @focusin="focusPane(tab.id, pane.id)"
    >
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
