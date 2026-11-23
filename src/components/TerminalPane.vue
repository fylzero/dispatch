<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { Terminal, type ITheme } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { openUrl } from "@tauri-apps/plugin-opener";
import * as api from "../api";
import { fontFaceName, resolveFontStack } from "../fonts";
import { isMac, isWindows } from "../platform";
import { settings } from "../settings";
import { closePane, registerTerminal, setTerminalTitle } from "../tabs";
import { resolvedTheme } from "../theme";
import type { ResolvedTheme } from "../types";

const TERMINAL_THEMES: Record<ResolvedTheme, ITheme> = {
  dark: {
    background: "#08090c",
    foreground: "#e8edf5",
    cursor: "#3dd6b4",
    cursorAccent: "#08090c",
    selectionBackground: "#2a3344",
    black: "#1c2129",
    red: "#f07178",
    green: "#3dd68c",
    yellow: "#e6c07b",
    blue: "#7aa2f7",
    magenta: "#c678dd",
    cyan: "#56b6c2",
    white: "#d7deea",
    brightBlack: "#66707f",
    brightRed: "#ff8b93",
    brightGreen: "#5ee6a3",
    brightYellow: "#f0d090",
    brightBlue: "#9ab8ff",
    brightMagenta: "#d899ea",
    brightCyan: "#7ccbd5",
    brightWhite: "#ffffff",
  },
  light: {
    background: "#ffffff",
    foreground: "#1c2330",
    cursor: "#118777",
    cursorAccent: "#ffffff",
    selectionBackground: "#cfe0f7",
    black: "#1c2330",
    red: "#c9303a",
    green: "#15803d",
    yellow: "#a16207",
    blue: "#2c5fcf",
    magenta: "#9333ea",
    cyan: "#0e7490",
    white: "#6e7781",
    brightBlack: "#5f6b7d",
    brightRed: "#d1434b",
    brightGreen: "#178a52",
    brightYellow: "#b07a12",
    brightBlue: "#2f6fdb",
    brightMagenta: "#a855f7",
    brightCyan: "#0891b2",
    brightWhite: "#8c959f",
  },
};

/**
 * xterm draws on a clear background so the window opacity shows through; it
 * still uses the solid colour for inverse text (via its own opaque()).
 */
function themeFor(theme: ResolvedTheme): ITheme {
  const colors = TERMINAL_THEMES[theme];
  return { ...colors, background: `${colors.background}00` };
}

/** A shell that quits this soon after starting likely failed; keep its tab so the error shows. */
const EARLY_EXIT_MS = 1500;

const props = defineProps<{ id: string; active: boolean }>();

const host = ref<HTMLDivElement | null>(null);

let term: Terminal | null = null;
let fit: FitAddon | null = null;
let sessionId = "";
let startedAt = 0;
let resizeObserver: ResizeObserver | null = null;
let unregister: (() => void) | undefined;

function syncSize() {
  if (!term || !fit || !host.value?.clientWidth) return;
  fit.fit();
  if (sessionId) {
    void api.resizeTerminal(sessionId, term.cols, term.rows).catch(() => {
      /* closed while resizing */
    });
  }
}

async function preloadFont() {
  const face = fontFaceName(resolveFontStack(settings.value.fontFamily));
  try {
    if (face) await document.fonts.load(`${settings.value.fontSize}px "${face}"`);
    await document.fonts.ready;
  } catch {
    /* use the fallback stack if the face is not available */
  }
}

function onExit() {
  sessionId = "";
  if (Date.now() - startedAt < EARLY_EXIT_MS) {
    term?.writeln("\r\n\x1b[2m[Process exited]\x1b[0m");
    return;
  }
  closePane(props.id);
}

async function start() {
  if (!term) return;
  startedAt = Date.now();
  try {
    sessionId = await api.openTerminal({
      cols: term.cols,
      rows: term.rows,
      shell: settings.value.shell,
      onData: (bytes) => term?.write(bytes),
      onExit,
    });
  } catch (err) {
    term.writeln(`\x1b[31m${api.errorMessage(err)}\x1b[0m`);
  }
}

/**
 * Keys the page handles instead of the shell. On Windows and Linux,
 * Ctrl+Shift+C/V copy and paste, so plain Ctrl+C stays an interrupt.
 */
function onKey(event: KeyboardEvent) {
  if (event.type !== "keydown" || isMac || !event.ctrlKey || !event.shiftKey) return true;
  const key = event.key.toLowerCase();
  if (key === "c") {
    const text = term?.getSelection();
    if (text) void navigator.clipboard.writeText(text);
    return false;
  }
  if (key === "v") {
    void navigator.clipboard.readText().then((text) => term?.paste(text));
    return false;
  }
  return true;
}

onMounted(async () => {
  const node = host.value;
  if (!node) return;
  await preloadFont();
  const s = settings.value;
  term = new Terminal({
    cursorBlink: s.cursorBlink,
    cursorStyle: s.cursorStyle,
    fontFamily: resolveFontStack(s.fontFamily),
    fontSize: s.fontSize,
    lineHeight: s.lineHeight,
    fontWeight: "400",
    fontWeightBold: "700",
    scrollback: s.scrollback,
    theme: themeFor(resolvedTheme.value),
    macOptionIsMeta: true,
    macOptionClickForcesSelection: true,
    /* The pane paints the (possibly see-through) background; xterm stays clear on top of it. */
    allowTransparency: true,
    /* ConPTY reflows lines itself; telling xterm avoids doubled wrapping on Windows. */
    ...(isWindows ? { windowsPty: { backend: "conpty" as const } } : {}),
  });
  fit = new FitAddon();
  term.loadAddon(fit);
  term.loadAddon(
    new WebLinksAddon((event, uri) => {
      /* Command-click (Ctrl-click elsewhere) opens links, like most terminals. */
      if (isMac ? event.metaKey : event.ctrlKey) void openUrl(uri).catch(() => undefined);
    }),
  );
  term.attachCustomKeyEventHandler(onKey);
  term.open(node);
  term.onData((data) => {
    if (sessionId) void api.writeTerminal(sessionId, data).catch(() => undefined);
  });
  term.onTitleChange((title) => setTerminalTitle(props.id, title));

  unregister = registerTerminal(props.id, {
    focus: () => term?.focus(),
    clear: () => term?.clear(),
    selectAll: () => term?.selectAll(),
  });

  resizeObserver = new ResizeObserver(() => syncSize());
  resizeObserver.observe(node);
  await nextTick();
  syncSize();
  await start();
  if (props.active) term.focus();
});

onUnmounted(() => {
  unregister?.();
  resizeObserver?.disconnect();
  if (sessionId) void api.closeTerminal(sessionId).catch(() => undefined);
  sessionId = "";
  term?.dispose();
  term = null;
  fit = null;
});

watch(
  () => props.active,
  async (active) => {
    if (!active || !term) return;
    await nextTick();
    syncSize();
    term.focus();
  },
);

watch(
  () => [settings.value.fontFamily, settings.value.fontSize, settings.value.lineHeight] as const,
  async ([family, size, lineHeight]) => {
    if (!term) return;
    await preloadFont();
    term.options.fontFamily = resolveFontStack(family);
    term.options.fontSize = size;
    term.options.lineHeight = lineHeight;
    syncSize();
  },
);

watch(
  () => [settings.value.cursorStyle, settings.value.cursorBlink, settings.value.scrollback] as const,
  ([cursorStyle, cursorBlink, scrollback]) => {
    if (!term) return;
    term.options.cursorStyle = cursorStyle;
    term.options.cursorBlink = cursorBlink;
    term.options.scrollback = scrollback;
  },
);

watch(resolvedTheme, (theme) => {
  if (term) term.options.theme = themeFor(theme);
});
</script>

<template>
  <div class="terminal-pane">
    <div ref="host" class="terminal-host" />
  </div>
</template>
