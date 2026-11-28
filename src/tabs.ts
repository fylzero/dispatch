import { computed, nextTick, ref, toRaw } from "vue";
import { homeDir } from "@tauri-apps/api/path";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  type Axis,
  type Direction,
  type LayoutNode,
  countPanes,
  firstPane,
  layout,
  neighborOf,
  removePane,
  siblingsAfterSplit,
  splitPane,
} from "./splits";
import * as api from "./api";
import { LABEL_COLORS } from "./colors";
import { isWindows } from "./platform";
import { settings } from "./settings";
import { showToast } from "./toast";
import type { LayoutTemplateNode, LayoutTemplateTab, SavedLayout } from "./types";

export const SETTINGS_TAB_ID = "settings";
export const CHANGELOG_TAB_ID = "changelog";

export const SETTINGS_SECTIONS = ["general", "appearance", "terminal", "hotkeys", "json"] as const;
export type SettingsSection = (typeof SETTINGS_SECTIONS)[number];

/** Panes one split can hold side by side (⌘D). */
export const MAX_SIDE_BY_SIDE = 8;
/** Panes one split can hold stacked (⌘⇧D). */
export const MAX_STACKED = 12;
/** Smallest a pane may become, in pixels, from splitting or dragging. */
export const MIN_PANE_WIDTH = 80;
export const MIN_PANE_HEIGHT = 48;

/**
 * True while a split divider is being dragged. Terminals keep their rows and
 * columns until it is let go, so the shells don't redraw on every pixel.
 */
export const dividerDragging = ref(false);

export interface AppTab {
  id: string;
  title: string;
  kind: "terminal" | "settings" | "changelog";
  /** A color id from LABEL_COLORS, set from the tab's right-click menu. */
  color?: string;
  /** True when the title is a name someone typed (the tab's or its pane's), not the shell's title. */
  named?: boolean;
}

/** A terminal tab: a tree of split panes and the one that has focus. */
export interface TerminalTab {
  id: string;
  root: LayoutNode;
  activePane: string;
  /** A name given by renaming the tab; without one it follows the focused pane. */
  name?: string;
  /** A color id from LABEL_COLORS. */
  color?: string;
}

/** What the menu can ask of a terminal pane. */
export interface TerminalHandle {
  focus(): void;
  clear(): void;
  selectAll(): void;
  /** Types text into the shell as if from the keyboard (not a bracketed paste, which zsh highlights). */
  type(text: string): void;
  /** The folder the pane's shell is in now, when it can be told. */
  cwd(): Promise<string | null>;
}

const terminals = ref<TerminalTab[]>([]);
const paneTitles = ref<Record<string, string>>({});
/** Names given to panes from their right-click menu; shown on the pane and used as its title. */
const paneNames = ref<Record<string, string>>({});
/** Color ids given to panes from their right-click menu. */
const paneColors = ref<Record<string, string>>({});
/** The tab or pane whose name is being edited in place. */
export const renamingTab = ref<string | null>(null);
export const renamingPane = ref<string | null>(null);
const settingsTabOpen = ref(false);
const changelogTabOpen = ref(false);
export const settingsSection = ref<SettingsSection>("general");
export const activeId = ref("");

const handles = new Map<string, TerminalHandle>();
/** Where a pane made by a split should start: the folder of the pane it split from. */
const startDirs = new Map<string, Promise<string | null>>();

/** The folder a new pane should start in, once; undefined for the default (a new tab). */
export function takeStartDir(paneId: string) {
  const dir = startDirs.get(paneId);
  startDirs.delete(paneId);
  return dir;
}
let nextTab = 1;
let nextId = 1;
const newId = (prefix: string) => `${prefix}-${nextId++}`;

function titleOf(paneId: string) {
  return paneNames.value[paneId] || paneTitles.value[paneId] || "Terminal";
}

export const tabs = computed<AppTab[]>(() => [
  ...terminals.value.map((tab) => ({
    id: tab.id,
    title: tab.name || titleOf(tab.activePane),
    kind: "terminal" as const,
    color: tab.color,
    named: Boolean(tab.name || paneNames.value[tab.activePane]),
  })),
  ...(settingsTabOpen.value ? [{ id: SETTINGS_TAB_ID, title: "Settings", kind: "settings" as const }] : []),
  ...(changelogTabOpen.value ? [{ id: CHANGELOG_TAB_ID, title: "Change Log", kind: "changelog" as const }] : []),
]);

/** Terminal tabs in the order they opened, so reordering tabs never moves their DOM. */
export const terminalTabs = computed(() =>
  [...terminals.value].sort((a, b) => a.id.localeCompare(b.id, undefined, { numeric: true })),
);

export const settingsOpen = computed(() => settingsTabOpen.value);
export const changelogOpen = computed(() => changelogTabOpen.value);

export function registerTerminal(paneId: string, handle: TerminalHandle) {
  handles.set(paneId, handle);
  return () => handles.delete(paneId);
}

/** The pane a file drag is over, for its outline. */
export const dropTargetPane = ref<string | null>(null);

/** The visible pane under a point in the window, if any. */
function paneAt(x: number, y: number) {
  const element = document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-pane-id]");
  const id = element?.dataset.paneId;
  return id && handles.has(id) ? id : null;
}

/**
 * Backslash-escapes a path so a shell reads it as one word, like Terminal.app.
 * Windows shells get double quotes instead, and a path with a newline gets single quotes.
 */
export function shellEscapePath(path: string) {
  if (isWindows) return /[\s&()^%!;,'"]/.test(path) ? `"${path}"` : path;
  if (/[\n\r]/.test(path)) return `'${path.replace(/'/g, `'\\''`)}'`;
  return path.replace(/[\s!"#$&'()*;<>?[\\\]^`{|}~]/g, "\\$&");
}

export function dragOver(x: number, y: number) {
  dropTargetPane.value = activeTab() ? (paneAt(x, y) ?? null) : null;
}

/** Types dropped files' paths into the pane under the cursor (or the focused one) and focuses it. */
export function dropPaths(paths: string[], x: number, y: number) {
  dropTargetPane.value = null;
  const tab = activeTab();
  if (!tab || !paths.length) return;
  const id = paneAt(x, y) ?? tab.activePane;
  const handle = handles.get(id);
  if (!handle) return;
  tab.activePane = id;
  handle.type(paths.map(shellEscapePath).join(" "));
  handle.focus();
}

function activeTab() {
  return terminals.value.find((tab) => tab.id === activeId.value);
}

/** The focused pane of the tab that is showing, if it is a terminal tab. */
export function activeTerminal() {
  const tab = activeTab();
  return tab ? handles.get(tab.activePane) : undefined;
}

export function activate(id: string) {
  activeId.value = id;
  const tab = terminals.value.find((item) => item.id === id);
  if (tab) void nextTick(() => handles.get(tab.activePane)?.focus());
}

export function focusPane(tabId: string, paneId: string) {
  const tab = terminals.value.find((item) => item.id === tabId);
  if (!tab || tab.activePane === paneId) return;
  tab.activePane = paneId;
}

/** The saved layout this window last opened or saved, shown with a check in the layouts menu. */
export const currentLayout = ref<string | null>(null);

/** The saved layout Dispatch opens with at launch, if one is set and still saved. */
export function defaultLayout(): SavedLayout | undefined {
  const { layouts, defaultLayout: name } = settings.value;
  return name ? layouts?.find((saved) => saved.name === name && saved.tabs?.length) : undefined;
}

const isColor = (value: unknown): value is string =>
  typeof value === "string" && LABEL_COLORS.some((color) => color.id === value);
const isName = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";
const weightOf = (value: unknown) => (typeof value === "number" && value > 0 && Number.isFinite(value) ? value : 1);

/** A layout tree from a template, with fresh ids, naming and coloring its panes. Bad entries become plain panes. */
function buildNode(template: LayoutTemplateNode | undefined): LayoutNode {
  const weight = weightOf(template?.weight);
  if (template?.kind === "split" && Array.isArray(template.children) && template.children.length) {
    const children = template.children.map(buildNode);
    if (children.length === 1) return { ...children[0], weight };
    return { kind: "split", id: newId("split"), axis: template.axis === "y" ? "y" : "x", weight, children };
  }
  const id = newId("pane");
  const name = template?.kind === "pane" ? template.name : undefined;
  const color = template?.kind === "pane" ? template.color : undefined;
  const cwd = template?.kind === "pane" ? template.cwd : undefined;
  if (isName(cwd)) startDirs.set(id, Promise.resolve(cwd.trim()));
  if (isName(name)) paneNames.value = { ...paneNames.value, [id]: name.trim() };
  if (isColor(color)) paneColors.value = { ...paneColors.value, [id]: color };
  return { kind: "pane", id, weight };
}

function buildTab(template?: LayoutTemplateTab): TerminalTab {
  const root: LayoutNode = template ? buildNode(template.root) : { kind: "pane", id: newId("pane"), weight: 1 };
  root.weight = 1;
  const name = template?.name;
  const color = template?.color;
  return {
    id: `term-${nextTab++}`,
    root,
    activePane: firstPane(root),
    name: isName(name) ? name.trim() : undefined,
    color: isColor(color) ? color : undefined,
  };
}

/** A new terminal tab: one pane, or the panes of a layout's tab. */
export function newTerminal(template?: LayoutTemplateTab) {
  const tab = buildTab(template);
  terminals.value = [...terminals.value, tab];
  activeId.value = tab.id;
}

/**
 * What a window opens with: at launch (the first window) every tab of the
 * layout set to open at launch, if any; otherwise, and in every later
 * window, one plain tab.
 */
export function openStartTabs(launch: boolean) {
  const saved = launch ? defaultLayout() : undefined;
  if (!saved) {
    currentLayout.value = null;
    newTerminal();
    return;
  }
  openLayoutTabs(saved.tabs, false);
  currentLayout.value = saved.name;
}

/**
 * Opens a layout's tabs in this window, after the tabs already open or in
 * place of them (closing their shells), and shows the first one.
 */
export function openLayoutTabs(templates: LayoutTemplateTab[], replace: boolean) {
  if (!templates.length) return;
  const opened = templates.map(buildTab);
  if (replace) {
    forgetPanes(terminals.value.flatMap((tab) => layout(tab.root).panes.map((pane) => pane.id)));
    renamingTab.value = null;
    renamingPane.value = null;
  }
  terminals.value = [...(replace ? [] : terminals.value), ...opened];
  activate(opened[0].id);
}

/** A folder with the home folder written as `~`, so a layout still fits after moving to another account. */
function tildePath(path: string, home: string) {
  if (!home) return path;
  if (path === home) return "~";
  return path.startsWith(`${home}/`) ? `~${path.slice(home.length)}` : path;
}

async function templateOf(node: LayoutNode, home: string): Promise<LayoutTemplateNode> {
  if (node.kind === "split") {
    const children = await Promise.all(node.children.map((child) => templateOf(child, home)));
    return { kind: "split", axis: node.axis, weight: node.weight, children };
  }
  const name = paneNames.value[node.id];
  const color = paneColors.value[node.id];
  const dir = await (handles.get(node.id)?.cwd() ?? Promise.resolve(null)).catch(() => null);
  const cwd = dir ? tildePath(dir, home) : undefined;
  return {
    kind: "pane",
    weight: node.weight,
    ...(name ? { name } : {}),
    ...(color ? { color } : {}),
    ...(cwd ? { cwd } : {}),
  };
}

/** This window's terminal tabs as a layout template, in the order they are shown, with each pane's folder. */
export async function captureLayoutTabs(): Promise<LayoutTemplateTab[]> {
  const home = await homeDir()
    .then((dir) => dir.replace(/\/+$/, ""))
    .catch(() => "");
  return Promise.all(
    terminals.value.map(async (tab) => ({
      ...(tab.name ? { name: tab.name } : {}),
      ...(tab.color ? { color: tab.color } : {}),
      root: await templateOf(toRaw(tab.root), home),
    })),
  );
}


function terminalTab(id: string) {
  return terminals.value.find((tab) => tab.id === id);
}

function tabOfPane(paneId: string) {
  return terminals.value.find((tab) => layout(tab.root).panes.some((pane) => pane.id === paneId));
}

/** Names a tab; a blank name goes back to following the focused pane's title. */
export function renameTab(id: string, name: string) {
  const tab = terminalTab(id);
  if (tab) tab.name = name.trim() || undefined;
}

export function setTabColor(id: string, color: string | undefined) {
  const tab = terminalTab(id);
  if (tab) tab.color = color;
}

/** The name the tab was given, if any (not the shell's title). */
export function tabName(id: string) {
  return terminalTab(id)?.name ?? "";
}

export function paneName(paneId: string) {
  return paneNames.value[paneId] ?? "";
}

export function paneColor(paneId: string) {
  return paneColors.value[paneId];
}

/** Names a pane; a blank name removes it. */
export function renamePane(paneId: string, name: string) {
  const { [paneId]: _old, ...rest } = paneNames.value;
  const trimmed = name.trim();
  paneNames.value = trimmed ? { ...rest, [paneId]: trimmed } : rest;
}

export function setPaneColor(paneId: string, color: string | undefined) {
  const { [paneId]: _old, ...rest } = paneColors.value;
  paneColors.value = color ? { ...rest, [paneId]: color } : rest;
}

/** Puts the pane's shell back in focus, e.g. after a menu or a rename closes. */
export function focusTerminalPane(paneId?: string) {
  const id = paneId ?? activeTab()?.activePane;
  if (id) void nextTick(() => handles.get(id)?.focus());
}

/** Forgets the titles, names and colors of panes that have closed. */
function forgetPanes(ids: string[]) {
  const drop = (record: Record<string, string>) =>
    Object.fromEntries(Object.entries(record).filter(([id]) => !ids.includes(id)));
  paneTitles.value = drop(paneTitles.value);
  paneNames.value = drop(paneNames.value);
  paneColors.value = drop(paneColors.value);
}

export function setTerminalTitle(paneId: string, title: string) {
  paneTitles.value = { ...paneTitles.value, [paneId]: title };
}

/** Each tab's size in pixels, reported by its view, for the split size check. */
const tabSizes = new Map<string, { width: number; height: number }>();

export function setTabSize(tabId: string, width: number, height: number) {
  tabSizes.set(tabId, { width, height });
}

/** Whether every pane in `root` would still be usable at this tab's size. */
function fits(tabId: string, root: LayoutNode) {
  const size = tabSizes.get(tabId);
  if (!size) return true;
  return layout(root).panes.every(
    ({ rect }) => rect.w * size.width >= MIN_PANE_WIDTH && rect.h * size.height >= MIN_PANE_HEIGHT,
  );
}

/**
 * Splits the focused pane: "vertical" (⌘D) puts a new pane beside it,
 * "horizontal" (⌘⇧D) puts one below it. Only that pane is divided.
 */
export function splitActive(direction: "vertical" | "horizontal") {
  const tab = activeTab();
  if (tab) splitPaneIn(tab, tab.activePane, direction);
}

/** Splits a given pane, as from its right-click menu. */
export function splitPaneById(paneId: string, direction: "vertical" | "horizontal") {
  const tab = tabOfPane(paneId);
  if (tab) splitPaneIn(tab, paneId, direction);
}

function splitPaneIn(tab: TerminalTab, target: string, direction: "vertical" | "horizontal") {
  const axis: Axis = direction === "vertical" ? "x" : "y";
  const limit = axis === "x" ? MAX_SIDE_BY_SIDE : MAX_STACKED;
  if (siblingsAfterSplit(tab.root, target, axis) > limit) {
    showToast(`Up to ${limit} panes can sit ${axis === "x" ? "side by side" : "stacked"} in one split.`, "error");
    return;
  }
  const pane = newId("pane");
  /* Try it on a copy first, so a split that would leave a sliver changes nothing. */
  const copy: LayoutNode = JSON.parse(JSON.stringify(toRaw(tab.root)));
  const trial = splitPane(copy, target, axis, { kind: "pane", id: pane, weight: 1 }, newId("split"));
  if (!fits(tab.id, trial)) {
    showToast("There isn't room to split this pane. Make the window or pane bigger first.", "error");
    return;
  }
  /* The new pane opens in the folder the split pane's shell is in. */
  const from = handles.get(target);
  if (from) startDirs.set(pane, from.cwd().catch(() => null));
  tab.root = trial;
  tab.activePane = pane;
}

/** Closes one pane; the last pane in a tab closes the tab. */
export function closePane(paneId: string) {
  const tab = tabOfPane(paneId);
  if (!tab) return;
  if (countPanes(tab.root) > 1) forgetPanes([paneId]);
  const { root, focus } = removePane(tab.root, paneId);
  if (!root) {
    closeTab(tab.id);
    return;
  }
  tab.root = root;
  if (tab.activePane === paneId && focus) {
    tab.activePane = focus;
    if (activeId.value === tab.id) void nextTick(() => handles.get(focus)?.focus());
  }
}

/** Moves focus to the pane next to the focused one (⌘⌥ + arrow). */
export function focusNeighbor(direction: Direction) {
  const tab = activeTab();
  if (!tab) return;
  const target = neighborOf(tab.root, tab.activePane, direction);
  if (!target) return;
  tab.activePane = target;
  handles.get(target)?.focus();
}

export function openSettings(section: SettingsSection = settingsSection.value) {
  settingsTabOpen.value = true;
  settingsSection.value = section;
  activate(SETTINGS_TAB_ID);
}

export function openChangelog() {
  changelogTabOpen.value = true;
  activate(CHANGELOG_TAB_ID);
}

export function moveTab(id: string, targetId: string) {
  const from = terminals.value.findIndex((tab) => tab.id === id);
  const to = terminals.value.findIndex((tab) => tab.id === targetId);
  if (from === -1 || to === -1 || from === to) return;
  const next = [...terminals.value];
  const [tab] = next.splice(from, 1);
  next.splice(to, 0, tab);
  terminals.value = next;
}

export function isReorderable(id: string) {
  return terminals.value.some((tab) => tab.id === id);
}

/** Steps through the tabs as they are shown, wrapping at either end. */
export function cycleTab(step: 1 | -1) {
  const list = tabs.value;
  if (list.length < 2) return;
  const index = list.findIndex((tab) => tab.id === activeId.value);
  activate(list[(index + step + list.length) % list.length].id);
}

export function activateIndex(index: number) {
  const list = tabs.value;
  const tab = index === -1 ? list[list.length - 1] : list[index];
  if (tab) activate(tab.id);
}

/**
 * Closes a tab, with every pane in it, and shows its neighbour. With nothing
 * left the window closes; closing the last window quits the app, the same as
 * in Terminal. When Dispatch keeps running in the background the last window
 * hides instead, with a fresh tab ready for the next time it is shown.
 */
export function closeTab(id: string) {
  const list = tabs.value;
  const index = list.findIndex((tab) => tab.id === id);
  if (index === -1) return;
  if (id === SETTINGS_TAB_ID) settingsTabOpen.value = false;
  else if (id === CHANGELOG_TAB_ID) changelogTabOpen.value = false;
  else {
    const closing = terminalTab(id);
    if (closing) forgetPanes(layout(closing.root).panes.map((pane) => pane.id));
    terminals.value = terminals.value.filter((tab) => tab.id !== id);
  }
  if (renamingTab.value === id) renamingTab.value = null;

  const remaining = tabs.value;
  if (!remaining.length) {
    void api
      .closeWindow()
      .then((hidden) => hidden && openStartTabs(false))
      .catch(() => getCurrentWindow().close());
    return;
  }
  if (activeId.value === id) {
    const neighbor = remaining[index] ?? remaining[index - 1];
    activate(neighbor.id);
  }
}

/** ⌘W: the focused pane if the tab is split, otherwise the whole tab. */
export function closeActive() {
  const tab = activeTab();
  const paneCount = tab ? countPanes(tab.root) : 0;
  if (tab && paneCount > 1) closePane(tab.activePane);
  else closeTab(activeId.value);
}
