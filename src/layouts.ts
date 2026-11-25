/**
 * Saved layouts: named sets of terminal tabs, with their split panes, the
 * names and colors of both, and how big each pane is. They live in
 * settings.json, so every window shares them, and one can be set to open at
 * launch, in the first window Dispatch opens.
 */
import { computed } from "vue";
import { settings, updateSettings } from "./settings";
import { captureLayoutTabs, currentLayout, openLayoutTabs } from "./tabs";
import { showToast } from "./toast";
import type { SavedLayout } from "./types";

export const MAX_LAYOUTS = 20;
export const MAX_LAYOUT_NAME = 40;

export const savedLayouts = computed<SavedLayout[]>(() =>
  (settings.value.layouts ?? []).filter((saved) => saved && typeof saved.name === "string" && Array.isArray(saved.tabs)),
);

const sameName = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

export function findLayout(name: string) {
  return savedLayouts.value.find((saved) => sameName(saved.name, name));
}

/** Whether a layout with this name can be saved: an existing one is replaced, a new one needs a free slot. */
export function canSaveLayout(name: string) {
  return !!findLayout(name) || savedLayouts.value.length < MAX_LAYOUTS;
}

/**
 * Saves this window's tabs under a name, replacing a layout with the same
 * name, and sets whether Dispatch opens it at launch.
 */
export function saveLayout(rawName: string, openAtLaunch: boolean) {
  const name = rawName.trim().slice(0, MAX_LAYOUT_NAME);
  if (!name) return false;
  const tabs = captureLayoutTabs();
  if (!tabs.length) {
    showToast("Open a terminal tab first; a layout is made of terminal tabs.", "error");
    return false;
  }
  const existing = findLayout(name);
  if (!existing && savedLayouts.value.length >= MAX_LAYOUTS) {
    showToast(`Up to ${MAX_LAYOUTS} layouts can be saved. Delete one first.`, "error");
    return false;
  }
  const saved: SavedLayout = { name, tabs };
  const layouts = existing
    ? savedLayouts.value.map((item) => (item === existing ? saved : item))
    : [...savedLayouts.value, saved];
  const wasDefault = !!existing && isDefaultLayout(existing.name);
  const defaultLayout = openAtLaunch ? name : wasDefault ? "" : settings.value.defaultLayout;
  updateSettings({ layouts, defaultLayout });
  currentLayout.value = name;
  const said = existing ? `Updated the layout “${name}”` : `Saved the layout “${name}”`;
  showToast(openAtLaunch ? `${said}. Dispatch opens it at launch.` : `${said}.`);
  return true;
}

/** Opens a saved layout's tabs in this window, beside the open tabs or in place of them (closing their shells). */
export function applyLayout(name: string, replace: boolean) {
  const saved = findLayout(name);
  if (!saved?.tabs.length) return;
  openLayoutTabs(saved.tabs, replace);
  currentLayout.value = saved.name;
}

/** Whether a layout can be renamed to this: not blank and not another layout's name. */
export function canRenameLayout(from: string, to: string) {
  const target = findLayout(to);
  return !!to.trim() && (!target || target === findLayout(from));
}

/**
 * Edits a saved layout's name and whether Dispatch opens it at launch. Its
 * tabs stay as saved, and its check in this window follows the new name.
 */
export function editLayout(from: string, rawName: string, openAtLaunch: boolean) {
  const name = rawName.trim().slice(0, MAX_LAYOUT_NAME);
  const saved = findLayout(from);
  if (!saved || !canRenameLayout(from, name)) return false;
  const wasDefault = isDefaultLayout(saved.name);
  updateSettings({
    layouts: savedLayouts.value.map((item) => (item === saved ? { ...item, name } : item)),
    defaultLayout: openAtLaunch ? name : wasDefault ? "" : settings.value.defaultLayout,
  });
  if (currentLayout.value && sameName(currentLayout.value, saved.name)) currentLayout.value = name;
  showToast(`Saved the changes to “${name}”.`);
  return true;
}

export function deleteLayout(name: string) {
  const saved = findLayout(name);
  if (!saved) return;
  const wasDefault = sameName(settings.value.defaultLayout, saved.name);
  updateSettings({
    layouts: savedLayouts.value.filter((item) => item !== saved),
    defaultLayout: wasDefault ? "" : settings.value.defaultLayout,
  });
  if (currentLayout.value && sameName(currentLayout.value, saved.name)) currentLayout.value = null;
  showToast(`Deleted the layout “${saved.name}”.`);
}

export function isDefaultLayout(name: string) {
  return !!settings.value.defaultLayout && sameName(settings.value.defaultLayout, name);
}
