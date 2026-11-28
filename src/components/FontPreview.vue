<script setup lang="ts">
/*
 * A three-line sample in a real (read-only) xterm, with the same options and
 * prompt chips as a tab, so it looks exactly like the terminal. It follows
 * the font settings and the prompt style new tabs use: the Dispatch prompt's
 * chips, with the command on its own line or after them, or a plain prompt.
 *
 * It is see-through like a pane: it paints the terminal color at the window
 * opacity, and the settings page and card behind it get a matching hole, so
 * the desktop (and the window's blur) shows through.
 */
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { fontFaceName, resolveFontStack } from "../fonts";
import { CHIP_ICONS } from "../promptChips";
import { settings } from "../settings";
import { resolvedTheme, TERMINAL_THEMES } from "../theme";
import type { ResolvedTheme } from "../types";

const ESC = "\x1b";
const GREEN = `${ESC}[32m`;
const YELLOW = `${ESC}[33m`;
const DIM = `${ESC}[90m`;
const RESET = `${ESC}[39m`;
const ROWS = 3;

const COMMITS = [
  { hash: "4f2a9c1", message: "Add font preview to settings" },
  { hash: "b81e07d", message: "Tidy the appearance page" },
];

interface Chip {
  kind: string;
  x: number;
  width: number;
}

const box = ref<HTMLElement | null>(null);
const host = ref<HTMLElement | null>(null);
let term: Terminal | null = null;
let observer: ResizeObserver | null = null;
let lastWidth = 0;
let building = 0;

/* Everything the sample depends on; any change rebuilds it. */
const look = computed(() => ({
  fontFamily: settings.value.fontFamily,
  fontSize: settings.value.fontSize,
  lineHeight: settings.value.lineHeight,
  theme: resolvedTheme.value,
  chips: settings.value.dispatchPrompt,
  below: settings.value.typeBelowPrompt,
}));

/**
 * The sample's lines as the shell would print them, and where its chips are.
 * Chips are laid out as the Dispatch prompt prints them (src-tauri/shell):
 * one cell of padding, two for the icon, the text, one more cell, then a space.
 */
function sample(chipsOn: boolean, below: boolean) {
  const commandBelow = chipsOn && below;
  const commits = COMMITS.slice(0, commandBelow ? 1 : 2);
  const command = `git log --oneline -${commits.length}`;
  const chips: Chip[] = [];
  let prompt = "";
  let x = 0;
  const chip = (kind: string, text: string, colored: string) => {
    chips.push({ kind, x, width: text.length });
    prompt += `${colored} `;
    x += text.length + 1;
  };
  if (chipsOn) {
    chip("cwd", "   ~/projects ", "   ~/projects ");
    chip("branch", "   main ", `   ${GREEN}main${RESET} `);
    chip("changes", " ± 2 ", ` ${YELLOW}± 2${RESET} `);
  } else {
    prompt = `${DIM}~/projects $${RESET} `;
  }
  const lines = commandBelow ? [prompt.trimEnd(), command] : [prompt + command];
  for (const commit of commits) lines.push(`${YELLOW}${commit.hash}${RESET} ${commit.message}`);
  const longest = Math.max(...lines.map((line) => line.replace(/\x1b\[\d+m/g, "").length));
  return { lines, chips, longest };
}

/* Matches the preview's width (at least its longest line) and exactly three rows. */
function fitSize(t: Terminal, longest: number) {
  const el = host.value;
  const screen = el?.querySelector<HTMLElement>(".xterm-screen");
  if (!el || !screen) return;
  const cellWidth = screen.clientWidth / t.cols;
  const fits = cellWidth > 0 ? Math.floor(el.clientWidth / cellWidth) : t.cols;
  const cols = Math.max(fits, longest);
  if (cols !== t.cols) t.resize(cols, ROWS);
  el.style.height = `${screen.clientHeight}px`;
}

/* Clear, like a pane's: the box behind it paints the see-through background. */
const clearTheme = (theme: ResolvedTheme) => ({
  ...TERMINAL_THEMES[theme],
  background: `${TERMINAL_THEMES[theme].background}00`,
});

let shape = "";
let longestLine = 0;

/*
 * Font, size, line height, and theme change the open terminal in place, so
 * dragging a slider doesn't redraw the sample from scratch. Only the prompt
 * style (what the lines say) starts a new one.
 */
async function build() {
  const el = host.value;
  if (!el) return;
  const run = ++building;
  const s = look.value;
  const stack = resolveFontStack(s.fontFamily);
  const face = fontFaceName(stack);
  try {
    if (face) await document.fonts.load(`${s.fontSize}px "${face}"`);
    await document.fonts.ready;
  } catch {
    /* use the fallback stack if the face is not available */
  }
  if (run !== building || !host.value) return;

  const nextShape = `${s.chips}|${s.below}`;
  if (term && shape === nextShape) {
    const t = term;
    t.options.fontFamily = stack;
    t.options.fontSize = s.fontSize;
    t.options.lineHeight = s.lineHeight;
    t.options.theme = clearTheme(s.theme);
    /* xterm measures the new cell size before its next frame. */
    requestAnimationFrame(() => {
      if (term === t) fitSize(t, longestLine);
    });
    return;
  }

  term?.dispose();
  shape = nextShape;
  const { lines, chips, longest } = sample(s.chips, s.below);
  longestLine = longest;
  const t = new Terminal({
    cols: Math.max(longest, 20),
    rows: ROWS,
    fontFamily: stack,
    fontSize: s.fontSize,
    lineHeight: s.lineHeight,
    fontWeight: "400",
    fontWeightBold: "700",
    scrollback: 0,
    disableStdin: true,
    cursorBlink: false,
    theme: clearTheme(s.theme),
    allowTransparency: true,
    allowProposedApi: true,
  });
  term = t;
  t.open(el);
  fitSize(t, longest);

  t.write(`${ESC}[?25l${lines.join("\r\n")}`, () => {
    if (term !== t) return;
    const top = t.registerMarker(-t.buffer.active.cursorY);
    if (!top) return;
    for (const chip of chips) {
      const decoration = t.registerDecoration({ marker: top, x: chip.x, width: chip.width });
      decoration?.onRender((node) => {
        node.style.setProperty("--cell", `${parseFloat(node.style.width) / chip.width}px`);
        if (node.dataset.chip) return;
        node.dataset.chip = chip.kind;
        node.classList.add("prompt-chip", `chip-${chip.kind}`);
        const icon = CHIP_ICONS[chip.kind];
        if (icon) node.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true">${icon}</svg>`;
      });
    }
  });
}

watch(look, () => void build());

/*
 * The hole: the settings page and card paint their backgrounds in a ::before
 * masked by a rounded rectangle where this box is (styles.css, .see-through).
 * Only its place and size change; the mask image itself never does, since
 * WebKit hides the whole layer for a frame while a new mask image loads.
 * Placed straight from the resize and scroll events, which come before the
 * frame is painted, so the hole never trails the box.
 */
let behind: HTMLElement[] = [];

function placeHole() {
  const el = box.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  for (const layer of behind) {
    const area = layer.getBoundingClientRect();
    layer.style.setProperty("--hole-x", `${rect.left - area.left - layer.clientLeft}px`);
    layer.style.setProperty("--hole-y", `${rect.top - area.top - layer.clientTop}px`);
    layer.style.setProperty("--hole-w", `${rect.width}px`);
    layer.style.setProperty("--hole-h", `${rect.height}px`);
  }
}

let holeObserver: ResizeObserver | null = null;

onMounted(() => {
  void build();
  const el = box.value;
  if (el) {
    behind = [el.closest<HTMLElement>(".main-pane"), el.closest<HTMLElement>(".settings-card")].filter(
      (layer): layer is HTMLElement => !!layer,
    );
    for (const layer of behind) layer.classList.add("see-through");
    holeObserver = new ResizeObserver(placeHole);
    holeObserver.observe(el);
    for (const layer of behind) holeObserver.observe(layer);
    /* Capture catches the settings page scrolling, which moves the box. */
    window.addEventListener("scroll", placeHole, true);
    window.addEventListener("resize", placeHole);
    placeHole();
  }
  observer = new ResizeObserver(([entry]) => {
    const width = Math.round(entry.contentRect.width);
    if (width === lastWidth) return;
    const first = lastWidth === 0;
    lastWidth = width;
    if (!first) void build();
  });
  if (host.value) observer.observe(host.value);
});

onUnmounted(() => {
  building++;
  holeObserver?.disconnect();
  window.removeEventListener("scroll", placeHole, true);
  window.removeEventListener("resize", placeHole);
  for (const layer of behind) {
    layer.classList.remove("see-through");
    for (const name of ["--hole-x", "--hole-y", "--hole-w", "--hole-h"]) layer.style.removeProperty(name);
  }
  behind = [];
  observer?.disconnect();
  term?.dispose();
  term = null;
});
</script>

<template>
  <div ref="box" class="font-preview" aria-hidden="true">
    <div ref="host" class="font-preview-host" />
  </div>
</template>

<style scoped>
.font-preview {
  padding: 0.6rem 0.75rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
  /* The same see-through background as a pane. */
  background: rgb(var(--terminal-rgb) / var(--terminal-opacity, 1));
}

/* Read-only: no text cursor or selection, like a picture of a tab. */
.font-preview-host {
  pointer-events: none;
  overflow: hidden;
}

.font-preview-host :deep(.xterm-viewport) {
  overflow: hidden !important;
  background-color: transparent !important;
}
</style>
