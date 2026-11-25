<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { Terminal, type IDecoration, type IMarker, type ITheme } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { Menu } from "@tauri-apps/api/menu";
import { homeDir } from "@tauri-apps/api/path";
import { openUrl } from "@tauri-apps/plugin-opener";
import * as api from "../api";
import { fontFaceName, resolveFontStack } from "../fonts";
import { isMac, isWindows } from "../platform";
import { settings } from "../settings";
import { closePane, dividerDragging, registerTerminal, setTerminalTitle, takeStartDir } from "../tabs";
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

/*
 * The Dispatch prompt (src-tauri/shell) prints each chip as plain text wrapped
 * in OSC 6973 marks: "chip;<kind>" where it starts and "end" where it stops.
 * The text stays ordinary terminal text; a decoration over those cells draws
 * the chip's border and icon.
 */
const PROMPT_OSC = 6973;
const CHIP_ICONS: Record<string, string> = {
  node: '<path d="M12 2.5 20.5 7.4v9.2L12 21.5l-8.5-4.9V7.4Z"/><path class="glyph" d="M10.2 9.5v4.4c0 1-.6 1.6-1.5 1.6M16.2 10.3c-.3-.6-.9-.9-1.6-.9-.9 0-1.5.5-1.5 1.2 0 1.7 3.2 1 3.2 2.9 0 .8-.7 1.3-1.6 1.3-.8 0-1.4-.4-1.7-1"/>',
  cwd: '<path d="M20 19a2 2 0 0 0 2-2V8.5a2 2 0 0 0-2-2h-7.6a2 2 0 0 1-1.7-.9l-.8-1.2A2 2 0 0 0 8.2 3.5H4a2 2 0 0 0-2 2V17a2 2 0 0 0 2 2Z"/>',
  branch: '<path d="M6 3v12"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>',
  changes: "",
};

interface PromptChip {
  kind: string;
  marker: IMarker;
  x: number;
  width: number;
  text: string;
  decoration: IDecoration;
}

let chipStart: { kind: string; marker: IMarker; x: number } | null = null;
let chips: PromptChip[] = [];

function lineText(line: number, start: number, end: number) {
  return term?.buffer.normal.getLine(line)?.translateToString(false, start, end) ?? "";
}

/**
 * Styles a chip's decoration each time it renders, and hides it once its
 * cells no longer hold the prompt text (cleared, or written over).
 */
function renderChip(chip: PromptChip, el: HTMLElement) {
  const intact = chip.marker.line >= 0 && lineText(chip.marker.line, chip.x, chip.x + chip.width) === chip.text;
  el.hidden = !intact;
  if (!intact) return;
  el.style.setProperty("--cell", `${parseFloat(el.style.width) / chip.width}px`);
  if (el.dataset.chip) return;
  el.dataset.chip = chip.kind;
  el.classList.add("prompt-chip", `chip-${chip.kind}`);
  const icon = CHIP_ICONS[chip.kind];
  if (icon) el.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true">${icon}</svg>`;
}

/** The chip at a point in the pane, in the screen or in the snapshot stuck to its top. */
function chipAt(clientX: number, clientY: number, target: EventTarget | null) {
  const screen = term?.element?.querySelector<HTMLElement>(".xterm-screen");
  const buffer = term?.buffer.active;
  if (!term || !screen || !buffer || buffer.type !== "normal") return null;
  const box = screen.getBoundingClientRect();
  const cellWidth = box.width / term.cols;
  const cellHeight = box.height / term.rows;
  const col = Math.floor((clientX - box.left) / cellWidth);
  let line: number;
  const stuck = sticky && !sticky.el.hidden && sticky.block && (target as Element | null)?.closest?.(".snapshot-sticky");
  if (stuck && sticky?.block) {
    line = sticky.block.marker.line + Math.floor((clientY - sticky.el.getBoundingClientRect().top) / cellHeight);
  } else {
    line = buffer.viewportY + Math.floor((clientY - box.top) / cellHeight);
  }
  return (
    chips.find(
      (chip) =>
        !chip.marker.isDisposed &&
        chip.marker.line === line &&
        col >= chip.x &&
        col < chip.x + chip.width &&
        lineText(line, chip.x, chip.x + chip.width) === chip.text,
    ) ?? null
  );
}

/** What Copy puts on the clipboard for a chip: the folder as a full path, or the branch name. */
async function chipValue(chip: PromptChip) {
  const text = chip.text.trim();
  if (chip.kind !== "cwd" || !(text === "~" || text.startsWith("~/"))) return text;
  const home = (await homeDir()).replace(/\/+$/, "");
  return home + text.slice(1);
}

function copyableChipAt(event: MouseEvent) {
  const chip = chipAt(event.clientX, event.clientY, event.target);
  return chip && (chip.kind === "cwd" || chip.kind === "branch") ? chip : null;
}

/* In WebKit xterm selects the word under a right click on mousedown; leave a chip's text alone. */
function onRightMouseDown(event: MouseEvent) {
  if (event.button === 2 && copyableChipAt(event)) event.stopPropagation();
}

/* Right-clicking the folder or branch chip offers to copy it. */
async function onContextMenu(event: MouseEvent) {
  const chip = copyableChipAt(event);
  if (!chip) return;
  event.preventDefault();
  event.stopPropagation();
  const value = await chipValue(chip);
  const menu = await Menu.new({
    items: [
      {
        id: "copy",
        text: chip.kind === "cwd" ? "Copy Path" : "Copy Branch Name",
        action: () => void navigator.clipboard.writeText(value).catch(() => undefined),
      },
    ],
  });
  await menu.popup();
}

function addChip(kind: string, marker: IMarker, x: number, width: number) {
  if (!term) return;
  /* zsh and bash redraw the prompt in place (resizing, Ctrl+L); keep one chip per spot. */
  chips = chips.filter((chip) => {
    const stale = chip.marker.isDisposed || (chip.marker.line === marker.line && chip.x === x);
    if (stale) chip.marker.dispose();
    return !stale;
  });
  const decoration = term.registerDecoration({ marker, x, width });
  if (!decoration) {
    marker.dispose();
    return;
  }
  const chip: PromptChip = { kind, marker, x, width, text: lineText(marker.line, x, x + width), decoration };
  decoration.onRender((el) => renderChip(chip, el));
  chips.push(chip);
}

/*
 * Blocks: the prompt also marks where it starts ("prompt"), where a command
 * starts ("exec"), and where it ends ("done;<exit code>"). Each prompt line
 * gets a border above it, so every command and its output sit in a block of
 * their own.
 *
 * Once a command runs, its prompt (the chips and the command) becomes a
 * snapshot: a muted card above the command's output, with a clock at its
 * right that counts up while the command runs and stops when it ends. When
 * the card scrolls past the top of the pane, a copy of it sticks there until
 * the next block's prompt pushes it off.
 */
interface Snapshot {
  /** The line just below the prompt: blank when the shell leaves a line before the output (see "spaced"). */
  end: IMarker;
  decoration: IDecoration;
  startedAt: number;
  /** 0 while the command runs. */
  endedAt: number;
  code: string;
  /** The clocks showing this command's time: on the card, and on the copy stuck to the top. */
  clocks: Set<HTMLElement>;
  /** A copy of the prompt's rows and chips, for the top of the pane. */
  copy?: HTMLElement;
  copyRows: number;
  /** The pane was resized (or the font changed) since the copy was drawn. */
  stale: boolean;
}

interface Block {
  marker: IMarker;
  decoration: IDecoration;
  snapshot?: Snapshot;
}

let blocks: Block[] = [];

/*
 * Keys the Dispatch prompt binds, used only while a prompt that says so
 * ("prompt;newline") is waiting for input. Anywhere else (a command running,
 * another prompt) these keys do what they always do.
 *
 * Shift+Enter adds a line to the command instead of running it: xterm sends
 * the same \r as Enter, so the pane sends CSI 13;2u instead.
 *
 * Select All selects just the command being typed, every line of it, starting
 * where the prompt's "input" mark says it does; Backspace or Delete then sends
 * CSI 127;9u, which clears the command.
 *
 * Option+Shift and Command+Shift with Left/Right select on the cursor's line
 * the way a macOS text field does: the selection is anchored at the cursor,
 * and each press moves its other end a word (stopping at punctuation) or all
 * the way to the line's start or end. Backspace or Delete then deletes those
 * characters with ordinary keys: Backspaces for text left of the cursor, or
 * Right arrows and then Backspaces for text right of it.
 */
const SHIFT_ENTER = "\x1b[13;2u";
const CLEAR_INPUT = "\x1b[127;9u";
/*
 * Option+Left/Right move by word. The Dispatch prompt binds xterm's own
 * sequences for them (words end at punctuation); anywhere else the pane sends
 * Esc b / Esc f, which shells and other readline programs know.
 */
const WORD_LEFT = { prompt: "\x1b[1;3D", other: "\x1bb" };
const WORD_RIGHT = { prompt: "\x1b[1;3C", other: "\x1bf" };
let promptBindsKeys = false;
let inputStart: { marker: IMarker; x: number } | null = null;

/**
 * The cursor's line, which may wrap onto more rows: it starts at cell `x` of
 * row `y` (after the prompt on a command's first line), and positions on it
 * are cells counted from the start of row `y`. `end` is where its text stops.
 */
interface LineSpan {
  y: number;
  x: number;
  end: number;
}

/** A selection the pane made: the whole command, or part of the cursor's line from `anchor` (the cursor) to `focus`. */
type InputSelection = { kind: "all" } | { kind: "range"; line: LineSpan; anchor: number; focus: number };

let inputSelection: InputSelection | null = null;
let selectingInput = false;

function setInputStart(x: number) {
  inputStart?.marker.dispose();
  inputStart = term ? { marker: term.registerMarker(0), x } : null;
}

function forgetInput() {
  inputStart?.marker.dispose();
  inputStart = null;
  inputSelection = null;
}

/** Selects `length` cells from (x, y) as `selection`, or clears the selection when empty. */
function selectCells(x: number, y: number, length: number, selection: InputSelection) {
  if (!term) return;
  selectingInput = true;
  if (length > 0) term.select(x, y, length);
  else term.clearSelection();
  selectingInput = false;
  inputSelection = length > 0 ? selection : null;
}

/** The cursor's line and the cursor's position on it. */
function cursorLine(): { line: LineSpan; cursor: number } | undefined {
  const buffer = term?.buffer.active;
  if (!term || !buffer || buffer.type !== "normal") return;
  const cursorY = buffer.baseY + buffer.cursorY;
  const inputY = inputStart && inputStart.marker.line >= 0 ? inputStart.marker.line : -1;
  let y = cursorY;
  while (y > inputY && y > 0 && buffer.getLine(y)?.isWrapped) y--;
  const x = y === inputY && inputStart ? inputStart.x : 0;
  let last = cursorY;
  while (buffer.getLine(last + 1)?.isWrapped) last++;
  const text = buffer.getLine(last)?.translateToString(true).length ?? 0;
  const cursor = (cursorY - y) * term.cols + buffer.cursorX;
  const end = Math.max((last - y) * term.cols + text, cursor);
  return { line: { y, x, end }, cursor };
}

/** The character in the cell at `pos` on `line`; "" for an empty cell or the second half of a wide character. */
function charAt(line: LineSpan, pos: number) {
  if (!term) return "";
  const row = term.buffer.active.getLine(line.y + Math.floor(pos / term.cols));
  return row?.getCell(pos % term.cols)?.getChars() ?? "";
}

/** Whether the character before `pos` is part of a word (letters, digits, _), looking past the halves of wide characters. */
function wordBefore(line: LineSpan, pos: number) {
  let at = pos - 1;
  while (at > line.x && charAt(line, at) === "") at--;
  return /[\p{L}\p{N}_]/u.test(charAt(line, at));
}

function wordAt(line: LineSpan, pos: number) {
  return /[\p{L}\p{N}_]/u.test(charAt(line, pos)) || (charAt(line, pos) === "" && pos > line.x && wordBefore(line, pos));
}

/** The start of the word before `pos`, as Option+Left moves. */
function previousWord(line: LineSpan, pos: number) {
  while (pos > line.x && !wordBefore(line, pos)) pos--;
  while (pos > line.x && wordBefore(line, pos)) pos--;
  return pos;
}

/** The end of the word after `pos`, as Option+Right moves. */
function nextWord(line: LineSpan, pos: number) {
  while (pos < line.end && !wordAt(line, pos)) pos++;
  while (pos < line.end && wordAt(line, pos)) pos++;
  return pos;
}

/** Moves the selection's free end with `move`, starting a new selection at the cursor if there is none on its line. */
function extendSelection(move: (line: LineSpan, focus: number) => number) {
  if (!term) return;
  let selection = inputSelection?.kind === "range" ? inputSelection : null;
  if (!selection) {
    const at = cursorLine();
    if (!at) return;
    selection = { kind: "range", line: at.line, anchor: at.cursor, focus: at.cursor };
  }
  const focus = move(selection.line, selection.focus);
  const from = Math.min(selection.anchor, focus);
  const to = Math.max(selection.anchor, focus);
  const { line } = selection;
  selectCells(from % term.cols, line.y + Math.floor(from / term.cols), to - from, { ...selection, focus });
}

/** Keys that delete a range selection in the shell. */
function deleteRange(selection: Extract<InputSelection, { kind: "range" }>) {
  const from = Math.min(selection.anchor, selection.focus);
  const to = Math.max(selection.anchor, selection.focus);
  let count = 0;
  for (let pos = from; pos < to; pos++) if (charAt(selection.line, pos) !== "") count++;
  const right = term?.modes.applicationCursorKeysMode ? "\x1bOC" : "\x1b[C";
  const back = "\x7f".repeat(count);
  return selection.focus < selection.anchor ? back : right.repeat(count) + back;
}

/** Where the command typed at the prompt starts and how many cells it covers, or nothing when not at a prompt. */
function typedInput() {
  const buffer = term?.buffer.active;
  if (!term || !buffer || !inputStart || buffer.type !== "normal") return;
  const startY = inputStart.marker.line;
  if (startY < 0) return;
  const startX = inputStart.x;
  let endY = startY;
  let endX = startX;
  for (let y = buffer.baseY + term.rows - 1; y >= startY; y--) {
    /* Trailing spaces can be left over from an edit (zsh draws a tab as spaces and only moves back over them). */
    const text = (buffer.getLine(y)?.translateToString(true) ?? "").replace(/\s+$/, "");
    const length = y === startY ? Math.max(text.length, startX) : text.length;
    if (length > (y === startY ? startX : 0)) {
      endY = y;
      endX = length;
      break;
    }
  }
  /* Spaces typed at the end are behind the cursor, so the command reaches at least that far. */
  const toText = (endY - startY) * term.cols + endX - startX;
  const toCursor = (buffer.baseY + buffer.cursorY - startY) * term.cols + buffer.cursorX - startX;
  return { startX, startY, length: Math.max(toText, toCursor) };
}

/** Selects the command typed so far. False when there is nothing typed to select. */
function selectInput() {
  const input = promptBindsKeys ? typedInput() : undefined;
  if (!input || input.length <= 0) return false;
  selectCells(input.startX, input.startY, input.length, { kind: "all" });
  return true;
}

function disposeSnapshot(block: Block) {
  block.snapshot?.decoration.dispose();
  block.snapshot?.end.dispose();
  block.snapshot = undefined;
}

function disposeBlock(block: Block) {
  disposeSnapshot(block);
  block.decoration.dispose();
  block.marker.dispose();
}

function lineIsBlank(line: number) {
  return line < 0 || lineText(line, 0, term?.cols ?? 0).trim() === "";
}

function renderBlock(block: Block, el: HTMLElement) {
  el.hidden = lineIsBlank(block.marker.line);
  if (el.hidden || !term) return;
  el.style.setProperty("--cell", el.style.width);
  el.style.setProperty("--row", el.style.height);
  el.style.setProperty("--cols", String(term.cols));
  el.classList.add("block-start");
  /*
   * A snapshot's card has a border of its own, and so does the line between
   * blocks after a command that printed nothing: the card's bottom border is
   * already there, one line up.
   */
  const previous = blocks[blocks.indexOf(block) - 1]?.snapshot;
  const noOutput = !!previous && previous.end.line + 2 >= block.marker.line && lineIsBlank(previous.end.line);
  el.classList.toggle("ran", !!block.snapshot || noOutput);
  /* The border goes in the blank line the prompt prints first; without one (an older prompt) it stays off the text above. */
  el.classList.toggle("gap", lineIsBlank(block.marker.line - 1));
}

function addBlock() {
  if (!term) return;
  const marker = term.registerMarker(0);
  /* A prompt redrawn in place (resizing, Ctrl+L) is still the same block. */
  const same = blocks.find((block) => block.marker.line === marker.line);
  if (same) {
    marker.dispose();
    return;
  }
  const decoration = term.registerDecoration({ marker, x: 0, width: 1 });
  if (!decoration) {
    marker.dispose();
    return;
  }
  const block: Block = { marker, decoration };
  decoration.onRender((el) => renderBlock(block, el));
  marker.onDispose(() => {
    disposeSnapshot(block);
    blocks = blocks.filter((other) => other !== block);
    schedulePin();
  });
  blocks.push(block);
}

/** Run time to the millisecond: 0.049s, 12.300s, 2m 05.120s, 1h 02m 05.120s. */
function formatElapsed(ms: number) {
  const total = Math.floor(Math.max(0, ms)) / 1000;
  if (total < 60) return `${total.toFixed(3)}s`;
  const minutes = Math.floor(total / 60);
  const seconds = (total - minutes * 60).toFixed(3).padStart(6, "0");
  if (minutes < 60) return `${minutes}m ${seconds}s`;
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m ${seconds}s`;
}

/**
 * Whether the shell left a blank line between the command and its output, as
 * the Dispatch prompt does. The card's bottom border runs through its middle;
 * without one (an older prompt) the border sits on the prompt's last row.
 */
function isSpaced(block: Block) {
  return !!block.snapshot && lineIsBlank(block.snapshot.end.line);
}

/** How many rows the prompt took: from the chips down to the last line of the command. */
function snapshotRows(block: Block) {
  return block.snapshot ? Math.max(1, block.snapshot.end.line - block.marker.line) : 0;
}

function makeClock(snapshot: Snapshot) {
  const clock = document.createElement("span");
  clock.className = "snapshot-time";
  snapshot.clocks.add(clock);
  return clock;
}

/** Shows the block's run time, and its exit code when it failed, on each of its clocks. */
function paintClocks(block: Block) {
  const snapshot = block.snapshot;
  if (!term || !snapshot) return;
  const failed = snapshot.endedAt > 0 && snapshot.code !== "" && snapshot.code !== "0";
  const elapsed = formatElapsed((snapshot.endedAt || performance.now()) - snapshot.startedAt);
  const text = `${failed ? `exit ${snapshot.code} · ` : ""}${elapsed}`;
  const line = block.marker.line;
  const cols = term.cols;
  /* Only over empty cells (and one more to its left), so it never covers a long prompt. */
  const fits = line >= 0 && lineText(line, Math.max(0, cols - text.length - 1), cols).trim() === "";
  for (const clock of snapshot.clocks) {
    if (clock.textContent !== text) clock.textContent = text;
    clock.hidden = !fits;
    clock.classList.toggle("failed", failed);
  }
}

/* While a command runs its clocks count up every frame. */
let clockFrame = 0;

function tickClocks() {
  clockFrame = 0;
  let running = false;
  for (const block of blocks) {
    if (!block.snapshot || block.snapshot.endedAt) continue;
    running = true;
    paintClocks(block);
  }
  if (running) clockFrame = requestAnimationFrame(tickClocks);
}

function renderSnapshot(block: Block, el: HTMLElement) {
  const snapshot = block.snapshot;
  el.hidden = !snapshot || lineIsBlank(block.marker.line);
  if (el.hidden || !term || !snapshot) return;
  /* xterm rounds a decoration's size to whole pixels; the card spans the screen's real width. */
  const screen = term.element?.querySelector<HTMLElement>(".xterm-screen");
  el.style.setProperty("--cell", screen ? `${screen.clientWidth / term.cols}px` : el.style.width);
  el.style.setProperty("--row", screen ? `${screen.clientHeight / term.rows}px` : el.style.height);
  el.style.setProperty("--cols", String(term.cols));
  el.style.setProperty("--rows", String(snapshotRows(block)));
  el.classList.toggle("spaced", isSpaced(block));
  if (el.classList.contains("snapshot-card")) return;
  el.classList.add("snapshot-card");
  el.append(makeClock(snapshot));
  paintClocks(block);
}

/** The block's command started: its prompt, from the chips to the cursor's line, becomes a snapshot. */
function startSnapshot(block: Block) {
  if (!term || block.snapshot) return;
  const end = term.registerMarker(0);
  const decoration = term.registerDecoration({ marker: block.marker, x: 0, width: 1, height: 1 });
  if (!decoration) {
    end.dispose();
    return;
  }
  block.snapshot = {
    end,
    decoration,
    startedAt: performance.now(),
    endedAt: 0,
    code: "",
    clocks: new Set(),
    copyRows: 0,
    stale: true,
  };
  decoration.onRender((el) => renderSnapshot(block, el));
  if (!clockFrame) clockFrame = requestAnimationFrame(tickClocks);
}

/** The block's command ended: its clocks stop, showing the exit code if it failed. */
function finishSnapshot(block: Block, code: string) {
  const snapshot = block.snapshot;
  if (!snapshot || snapshot.endedAt) return;
  snapshot.endedAt = performance.now();
  snapshot.code = code;
  paintClocks(block);
}

/** A chip drawn into a copy of the prompt, as renderChip draws it over the terminal. */
function chipElement(chip: PromptChip, top: number, cellWidth: number, cellHeight: number) {
  const el = document.createElement("div");
  el.className = `prompt-chip chip-${chip.kind}`;
  el.style.cssText = `top:${top}px;left:${chip.x * cellWidth}px;width:${chip.width * cellWidth}px;height:${cellHeight}px`;
  el.style.setProperty("--cell", `${cellWidth}px`);
  const icon = CHIP_ICONS[chip.kind];
  if (icon) el.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true">${icon}</svg>`;
  return el;
}

/**
 * Draws a copy of a snapshot's prompt (its rows' text, in their colours, and
 * its chips) from the buffer, for the top of the pane once the prompt itself
 * has scrolled past it. Each run of same-styled text is placed at its cell.
 */
function buildCopy(block: Block, cellWidth: number, cellHeight: number) {
  const snapshot = block.snapshot;
  const buffer = term?.buffer.normal;
  if (!term || !snapshot || !buffer) return;
  const start = block.marker.line;
  const rows = snapshotRows(block);
  const copy = document.createElement("div");
  /* The class picks up xterm's own styles for text and its palette colours. */
  copy.className = "xterm-rows snapshot-rows";
  for (let row = 0; row < rows; row++) {
    const line = buffer.getLine(start + row);
    if (!line) continue;
    let run: HTMLSpanElement | null = null;
    let style = "";
    for (let x = 0; x < term.cols; x++) {
      const cell = line.getCell(x);
      if (!cell || cell.getWidth() === 0) continue;
      const chars = cell.getChars() || " ";
      const color = cell.isFgPalette() ? `p${cell.getFgColor()}` : cell.isFgRGB() ? `r${cell.getFgColor()}` : "";
      const next = `${color}|${cell.isBold()}|${cell.isDim()}|${cell.isItalic()}`;
      if (!run || next !== style) {
        style = next;
        if (chars === " " && !color) {
          run = null;
          continue;
        }
        run = document.createElement("span");
        run.style.cssText = `top:${row * cellHeight}px;left:${x * cellWidth}px`;
        if (cell.isFgPalette()) run.classList.add(`xterm-fg-${cell.getFgColor()}`);
        else if (cell.isFgRGB()) run.style.color = `#${cell.getFgColor().toString(16).padStart(6, "0")}`;
        if (cell.isBold()) run.classList.add("xterm-bold");
        if (cell.isDim()) run.classList.add("xterm-dim");
        if (cell.isItalic()) run.classList.add("xterm-italic");
        copy.append(run);
      }
      run.textContent += chars;
    }
  }
  for (const chip of chips) {
    const line = chip.marker.line;
    if (chip.marker.isDisposed || line < start || line >= start + rows) continue;
    if (lineText(line, chip.x, chip.x + chip.width) !== chip.text) continue;
    copy.append(chipElement(chip, (line - start) * cellHeight, cellWidth, cellHeight));
  }
  snapshot.copy = copy;
  snapshot.copyRows = rows;
  snapshot.stale = false;
}

function markSnapshotsStale() {
  for (const block of blocks) if (block.snapshot) block.snapshot.stale = true;
}

/* The copy of a snapshot stuck to the top of the pane, and the block it belongs to. */
let sticky: { el: HTMLElement; block?: Block; copy?: HTMLElement; clock?: HTMLElement } | null = null;

function hideSticky() {
  if (!sticky || sticky.el.hidden) return;
  sticky.el.hidden = true;
  if (sticky.clock) sticky.block?.snapshot?.clocks.delete(sticky.clock);
  sticky.block = undefined;
  sticky.copy = undefined;
  sticky.clock = undefined;
}

/**
 * Sticks the snapshot whose prompt has scrolled past the top of the pane to
 * the top, while its output still shows. The next block's prompt pushes it
 * up and off as it reaches the top. `full` is false while the screen sits
 * lower to keep the prompt at the bottom (nothing has scrolled past the top).
 */
function layoutSticky(screen: HTMLElement, cellHeight: number, full: boolean, offset: number) {
  const buffer = term?.buffer.active;
  if (!term || !buffer || buffer.type !== "normal" || !full) return hideSticky();
  /*
   * The card reaches half a line above its first row (its top border runs
   * through the blank line there). Where the screen starts below the pane's
   * top (its padding, and the leftover strip), the copy sits low enough that
   * the whole card shows, top border at the pane's top edge, the same size
   * as in the pane. A card whose top has gone past that edge is the one stuck.
   */
  const paneTop = (host.value ? parseFloat(getComputedStyle(host.value).paddingTop) || 0 : 0) + offset;
  const rest = cellHeight / 2 - paneTop;
  let index = -1;
  for (let i = blocks.length - 1; i >= 0; i--) {
    const line = blocks[i].marker.line;
    if (line >= 0 && (line - buffer.viewportY) * cellHeight < rest) {
      index = i;
      break;
    }
  }
  const block = blocks[index];
  const snapshot = block?.snapshot;
  if (!block || !snapshot || lineIsBlank(block.marker.line)) return hideSticky();
  const cellWidth = screen.clientWidth / term.cols;
  if (snapshot.stale || !snapshot.copy) buildCopy(block, cellWidth, cellHeight);
  if (!snapshot.copy) return hideSticky();
  const height = snapshot.copyRows * cellHeight;
  const next = blocks[index + 1];
  /* The copy's bottom border meets the next prompt's top border, in the blank line above it, and moves up with it. */
  const nextBorder = next ? (next.marker.line - buffer.viewportY - 0.5) * cellHeight : Infinity;
  const spaced = isSpaced(block);
  const below = spaced ? cellHeight / 2 : 0;
  const y = Math.min(rest, nextBorder - height - below);
  if (y + height + below <= -paneTop) return hideSticky();

  if (!sticky || !sticky.el.isConnected) {
    const el = document.createElement("div");
    el.className = "snapshot-sticky";
    el.hidden = true;
    screen.append(el);
    sticky = { el };
  }
  const el = sticky.el;
  if (sticky.block !== block || sticky.copy !== snapshot.copy) {
    if (sticky.clock) sticky.block?.snapshot?.clocks.delete(sticky.clock);
    const card = document.createElement("div");
    card.className = "snapshot-card";
    const clock = makeClock(snapshot);
    card.append(clock);
    el.replaceChildren(snapshot.copy, card);
    sticky.block = block;
    sticky.copy = snapshot.copy;
    sticky.clock = clock;
    paintClocks(block);
  }
  el.style.setProperty("--cell", `${cellWidth}px`);
  el.style.setProperty("--row", `${cellHeight}px`);
  el.style.setProperty("--cols", String(term.cols));
  el.style.setProperty("--rows", String(snapshot.copyRows));
  el.classList.toggle("spaced", spaced);
  el.lastElementChild?.classList.toggle("spaced", spaced);
  el.style.transform = y ? `translateY(${y}px)` : "";
  el.hidden = false;
}

/** ED 2 and 3 (clear) wipe the screen; blocks on it are gone, even though their lines may fill up again. */
function onEraseDisplay(params: (number | number[])[]) {
  const mode = params[0];
  const buffer = term?.buffer.active;
  if (buffer?.type === "normal" && (mode === 2 || mode === 3)) {
    const top = mode === 3 ? 0 : buffer.baseY;
    for (const block of [...blocks]) {
      if (block.marker.line >= top) disposeBlock(block);
    }
  }
  return false;
}

function onPromptMark(data: string) {
  const buffer = term?.buffer.active;
  if (!term || !buffer || buffer.type !== "normal") return true;
  const [mark, kind] = data.split(";");
  const current = blocks[blocks.length - 1];
  if (mark === "prompt") {
    promptBindsKeys = kind === "newline";
    addBlock();
  } else if (mark === "input") {
    setInputStart(buffer.cursorX);
  } else if (mark === "exec") {
    promptBindsKeys = false;
    forgetInput();
    if (current) startSnapshot(current);
  } else if (mark === "done") {
    if (current) finishSnapshot(current, kind ?? "");
  } else if (mark === "chip" && kind in CHIP_ICONS) {
    chipStart?.marker.dispose();
    chipStart = { kind, marker: term.registerMarker(0), x: buffer.cursorX };
  } else if (mark === "end" && chipStart) {
    const start = chipStart;
    chipStart = null;
    const width = buffer.cursorX - start.x;
    /* A chip that wrapped onto the next line is left as plain text. */
    if (start.marker.line !== buffer.baseY + buffer.cursorY || width <= 0) start.marker.dispose();
    else addChip(start.kind, start.marker, start.x, width);
  }
  return true;
}

/*
 * Until output fills the pane, the screen sits lower so its last line in use
 * (the prompt, usually) is at the bottom, and output grows upward from it.
 * The pane fits a whole number of rows, so the screen also moves down by
 * what is left over, or that strip would look like an extra blank line under
 * the prompt. Only the drawing moves; the shell still sees an ordinary screen.
 */
let pinFrame = 0;

function pinToBottom() {
  pinFrame = 0;
  const el = term?.element;
  const buffer = term?.buffer.active;
  const screen = el?.querySelector<HTMLElement>(".xterm-screen");
  if (!term || !el || !buffer || !screen) return;
  const cellHeight = screen.clientHeight / term.rows;
  let offset = 0;
  let spare = 0;
  if (buffer.type === "normal" && buffer.viewportY === buffer.baseY) {
    let last = buffer.cursorY;
    for (let y = term.rows - 1; y > last; y--) {
      if (!lineIsBlank(buffer.baseY + y)) {
        last = y;
        break;
      }
    }
    spare = term.rows - 1 - last;
    /* Mid-drag the screen can be taller than the pane; keep its bottom in view. */
    const leftover = el.clientHeight - screen.clientHeight;
    const room = dividerDragging.value ? leftover : Math.max(0, leftover);
    offset = Math.floor(spare * cellHeight + room);
  }
  el.style.transform = offset ? `translateY(${offset}px)` : "";
  /* Where overflow: clip is missing, the pane can still scroll to show xterm's input; keep it put. */
  if (host.value && host.value.scrollTop) host.value.scrollTop = 0;
  layoutSticky(screen, cellHeight, spare === 0, offset);
}

function schedulePin() {
  if (!pinFrame) pinFrame = requestAnimationFrame(pinToBottom);
}

/*
 * Clicks in the pane outside xterm's own element (the space above a pinned
 * prompt, or the padding) would take focus from the terminal; keep it there.
 */
function onPaneMouseDown(event: MouseEvent) {
  if (!term?.element || term.element.contains(event.target as Node)) return;
  event.preventDefault();
  term.focus();
}

/*
 * While a divider is dragged the terminal keeps its size and only the pane
 * around it changes: the screen is re-pinned to the pane's bottom in the same
 * frame, so the text glides with the divider instead of shuddering as rows
 * come and go and the shell redraws. It fits once the divider is let go.
 */
function onHostResize() {
  if (!dividerDragging.value) return syncSize();
  cancelAnimationFrame(pinFrame);
  pinToBottom();
}

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
  try {
    /* A pane made by a split starts in the folder of the pane it split from. */
    const cwd = (await takeStartDir(props.id)) ?? undefined;
    if (!term) return;
    startedAt = Date.now();
    sessionId = await api.openTerminal({
      cols: term.cols,
      rows: term.rows,
      shell: settings.value.shell,
      cwd,
      onData: (bytes) => term?.write(bytes),
      onExit,
    });
  } catch (err) {
    term?.writeln(`\x1b[31m${api.errorMessage(err)}\x1b[0m`);
  }
}

function send(data: string) {
  if (sessionId) void api.writeTerminal(sessionId, data).catch(() => undefined);
}

/** Select All: just the command at a Dispatch prompt with something typed, otherwise everything. */
function selectAll() {
  if (selectInput()) return;
  /* At the prompt with nothing typed there is nothing to select, rather than the whole screen. */
  const input = typedInput();
  if (input && input.length <= 0) {
    inputSelection = null;
    term?.clearSelection();
    return;
  }
  term?.selectAll();
}

/**
 * macOS text-field keys, outside full-screen apps (which get xterm's usual keys):
 * Command+A selects (handled here, before xterm selects everything and the
 * menu's Select All then narrows it, which flashed), Command+Left/Right go to
 * the start or end of the line (Ctrl+A / Ctrl+E), Option+Left/Right move by
 * word, and with Shift both select to there instead. Returns what to do, or
 * nothing for any other key.
 */
function macTextKey(event: KeyboardEvent): (() => void) | undefined {
  if (!isMac || event.ctrlKey || term?.buffer.active.type !== "normal") return;
  const command = event.metaKey && !event.altKey && !event.shiftKey;
  const option = event.altKey && !event.metaKey && !event.shiftKey;
  const commandShift = event.metaKey && event.shiftKey && !event.altKey;
  const optionShift = event.altKey && event.shiftKey && !event.metaKey;
  if (commandShift && event.key === "ArrowLeft") return () => extendSelection((line) => line.x);
  if (commandShift && event.key === "ArrowRight") return () => extendSelection((line) => line.end);
  if (optionShift && event.key === "ArrowLeft") return () => extendSelection(previousWord);
  if (optionShift && event.key === "ArrowRight") return () => extendSelection(nextWord);
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  if (command && key === "a") return selectAll;
  const move = (data: string) => () => {
    inputSelection = null;
    term?.clearSelection();
    send(data);
  };
  if (command && key === "ArrowLeft") return move("\x01");
  if (command && key === "ArrowRight") return move("\x05");
  const words = promptBindsKeys ? "prompt" : "other";
  if (option && key === "ArrowLeft") return move(WORD_LEFT[words]);
  if (option && key === "ArrowRight") return move(WORD_RIGHT[words]);
}

/** Keys that only change the selection or copy it, which leave the command selected. */
function keepsInputSelected(event: KeyboardEvent) {
  return ["Shift", "Meta", "Control", "Alt", "CapsLock"].includes(event.key) || event.metaKey || (event.ctrlKey && event.shiftKey);
}

/**
 * Keys the page handles instead of the shell. On Windows and Linux,
 * Ctrl+Shift+C/V copy and paste, so plain Ctrl+C stays an interrupt. At a
 * Dispatch prompt, Shift+Enter adds a line and Backspace deletes the part of
 * the command the pane selected (see SHIFT_ENTER).
 */
function onKey(event: KeyboardEvent) {
  const atPrompt = promptBindsKeys && term?.buffer.active.type === "normal";
  const plain = !event.ctrlKey && !event.altKey && !event.metaKey;
  if (event.key === "Enter" && event.shiftKey && plain && atPrompt) {
    if (event.type === "keydown") send(SHIFT_ENTER);
    return false;
  }
  const mac = macTextKey(event);
  if (mac) {
    if (event.type === "keydown") mac();
    return false;
  }
  if (inputSelection && event.type === "keydown" && !keepsInputSelected(event)) {
    const selected = inputSelection;
    const remove = atPrompt && plain && !event.shiftKey && (event.key === "Backspace" || event.key === "Delete");
    inputSelection = null;
    term?.clearSelection();
    if (remove) {
      send(selected.kind === "all" ? CLEAR_INPUT : deleteRange(selected));
      return false;
    }
  }
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
    /* Decorations draw the prompt's chips. */
    allowProposedApi: true,
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
  term.parser.registerOscHandler(PROMPT_OSC, onPromptMark);
  term.onSelectionChange(() => {
    if (!selectingInput) inputSelection = null;
  });
  term.parser.registerCsiHandler({ final: "J" }, onEraseDisplay);
  term.onRender(schedulePin);
  term.onScroll(schedulePin);
  term.onResize(() => {
    markSnapshotsStale();
    schedulePin();
  });

  unregister = registerTerminal(props.id, {
    focus: () => term?.focus(),
    clear: () => {
      term?.clear();
      /* clear() keeps only the cursor's line; drop the blocks above it. */
      const line = term ? term.buffer.active.baseY + term.buffer.active.cursorY : 0;
      for (const block of [...blocks]) {
        if (block.marker.line !== line) disposeBlock(block);
      }
      schedulePin();
    },
    selectAll,
    type: (text) => term?.input(text),
    cwd: () => (sessionId ? api.terminalCwd(sessionId) : Promise.resolve(null)),
  });

  /* Capture, so xterm doesn't select the word under a chip first. */
  node.addEventListener("mousedown", onRightMouseDown, true);
  node.addEventListener("contextmenu", onContextMenu, true);
  resizeObserver = new ResizeObserver(onHostResize);
  resizeObserver.observe(node);
  await nextTick();
  syncSize();
  await start();
  if (props.active) term.focus();
});

onUnmounted(() => {
  unregister?.();
  resizeObserver?.disconnect();
  host.value?.removeEventListener("mousedown", onRightMouseDown, true);
  host.value?.removeEventListener("contextmenu", onContextMenu, true);
  if (sessionId) void api.closeTerminal(sessionId).catch(() => undefined);
  sessionId = "";
  chips = [];
  chipStart = null;
  blocks = [];
  sticky = null;
  cancelAnimationFrame(clockFrame);
  clockFrame = 0;
  promptBindsKeys = false;
  forgetInput();
  cancelAnimationFrame(pinFrame);
  pinFrame = 0;
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
    markSnapshotsStale();
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

/* Let go of a divider: fit the new size once. */
watch(dividerDragging, (dragging) => {
  if (!dragging) syncSize();
});

watch(resolvedTheme, (theme) => {
  if (term) term.options.theme = themeFor(theme);
});
</script>

<template>
  <div class="terminal-pane" @mousedown="onPaneMouseDown">
    <div ref="host" class="terminal-host" />
  </div>
</template>
