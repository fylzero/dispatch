import { computed, nextTick, ref, toRaw } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  type Axis,
  type Direction,
  type LayoutNode,
  countPanes,
  layout,
  neighborOf,
  removePane,
  siblingsAfterSplit,
  splitPane,
} from "./splits";
import { showToast } from "./toast";

export const SETTINGS_TAB_ID = "settings";
export const CHANGELOG_TAB_ID = "changelog";

export const SETTINGS_SECTIONS = ["general", "terminal"] as const;
export type SettingsSection = (typeof SETTINGS_SECTIONS)[number];

/** Panes one split can hold side by side (⌘D). */
export const MAX_SIDE_BY_SIDE = 8;
/** Panes one split can hold stacked (⌘⇧D). */
export const MAX_STACKED = 12;
/** Smallest a pane may become, in pixels, from splitting or dragging. */
export const MIN_PANE_WIDTH = 80;
export const MIN_PANE_HEIGHT = 48;

export interface AppTab {
  id: string;
  title: string;
  kind: "terminal" | "settings" | "changelog";
}

/** A terminal tab: a tree of split panes and the one that has focus. */
export interface TerminalTab {
  id: string;
  root: LayoutNode;
  activePane: string;
}

/** What the menu can ask of a terminal pane. */
export interface TerminalHandle {
  focus(): void;
  clear(): void;
  selectAll(): void;
}

const terminals = ref<TerminalTab[]>([]);
const paneTitles = ref<Record<string, string>>({});
const settingsTabOpen = ref(false);
const changelogTabOpen = ref(false);
export const settingsSection = ref<SettingsSection>("general");
export const activeId = ref("");

const handles = new Map<string, TerminalHandle>();
let nextTab = 1;
let nextId = 1;
const newId = (prefix: string) => `${prefix}-${nextId++}`;

function titleOf(paneId: string) {
  return paneTitles.value[paneId] || "Terminal";
}

export const tabs = computed<AppTab[]>(() => [
  ...terminals.value.map((tab) => ({ id: tab.id, title: titleOf(tab.activePane), kind: "terminal" as const })),
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

export function newTerminal() {
  const pane = newId("pane");
  const tab: TerminalTab = { id: `term-${nextTab++}`, root: { kind: "pane", id: pane, weight: 1 }, activePane: pane };
  terminals.value = [...terminals.value, tab];
  activeId.value = tab.id;
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
  if (!tab) return;
  const axis: Axis = direction === "vertical" ? "x" : "y";
  const limit = axis === "x" ? MAX_SIDE_BY_SIDE : MAX_STACKED;
  if (siblingsAfterSplit(tab.root, tab.activePane, axis) > limit) {
    showToast(`Up to ${limit} panes can sit ${axis === "x" ? "side by side" : "stacked"} in one split.`, "error");
    return;
  }
  const pane = newId("pane");
  /* Try it on a copy first, so a split that would leave a sliver changes nothing. */
  const copy: LayoutNode = JSON.parse(JSON.stringify(toRaw(tab.root)));
  const trial = splitPane(copy, tab.activePane, axis, { kind: "pane", id: pane, weight: 1 }, newId("split"));
  if (!fits(tab.id, trial)) {
    showToast("There isn't room to split this pane. Make the window or pane bigger first.", "error");
    return;
  }
  tab.root = trial;
  tab.activePane = pane;
}

/** Closes one pane; the last pane in a tab closes the tab. */
export function closePane(paneId: string) {
  const tab = terminals.value.find((item) => layout(item.root).panes.some((pane) => pane.id === paneId));
  if (!tab) return;
  const { [paneId]: _closed, ...rest } = paneTitles.value;
  paneTitles.value = rest;
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
 * left the window closes, which quits the app, the same as in Terminal.
 */
export function closeTab(id: string) {
  const list = tabs.value;
  const index = list.findIndex((tab) => tab.id === id);
  if (index === -1) return;
  if (id === SETTINGS_TAB_ID) settingsTabOpen.value = false;
  else if (id === CHANGELOG_TAB_ID) changelogTabOpen.value = false;
  else terminals.value = terminals.value.filter((tab) => tab.id !== id);

  const remaining = tabs.value;
  if (!remaining.length) {
    void getCurrentWindow().close();
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
