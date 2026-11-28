<script setup lang="ts">
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import LayoutMenu from "./LayoutMenu.vue";
import NameLabel from "./NameLabel.vue";
import { useUpdater } from "../composables/useUpdater";
import { colorRgb } from "../colors";
import { openContextMenu } from "../contextMenu";
import { isMac } from "../platform";
import {
  type AppTab,
  CHANGELOG_TAB_ID,
  SETTINGS_TAB_ID,
  activate,
  activeId,
  closeTab,
  isReorderable,
  moveTab,
  newTerminal,
  openChangelog,
  openSettings,
  renameTab,
  renamingTab,
  setTabColor,
  tabName,
  tabs,
} from "../tabs";

const { updateReady, availableVersion, showPrompt } = useUpdater();

const GITHUB_URL = "https://github.com/fylzero/dispatch";
const appVersion = ref("0.1.0");

void getVersion()
  .then((value) => {
    appVersion.value = value;
  })
  .catch(() => {
    /* keep the bundled fallback */
  });

async function openGitHub() {
  try {
    await openUrl(GITHUB_URL);
  } catch {
    /* stay quiet if the OS cannot open the URL */
  }
}

const newTabTitle = isMac ? "New tab (⌘T)" : "New tab (Ctrl+Shift+T)";

const DRAG_THRESHOLD = 5;
const tabList = ref<HTMLElement | null>(null);
const draggingId = ref<string | null>(null);
let pressed: { id: string; x: number } | null = null;

function startReorder(event: PointerEvent, id: string) {
  if (event.button !== 0 || !isReorderable(id) || (event.target as Element).closest("button, input")) return;
  event.preventDefault();
  pressed = { id, x: event.clientX };
  window.addEventListener("pointermove", onReorderMove);
  window.addEventListener("pointerup", endReorder);
  window.addEventListener("pointercancel", endReorder);
}

function onReorderMove(event: PointerEvent) {
  if (!pressed) return;
  if (!draggingId.value) {
    if (Math.abs(event.clientX - pressed.x) < DRAG_THRESHOLD) return;
    draggingId.value = pressed.id;
    activate(pressed.id);
  }
  const dragging = draggingId.value;
  const elements = Array.from(tabList.value?.querySelectorAll<HTMLElement>("[data-reorder-id]") ?? []);
  const target = elements.find((element) => {
    const rect = element.getBoundingClientRect();
    return event.clientX >= rect.left && event.clientX < rect.right;
  });
  const targetId = target?.dataset.reorderId;
  if (!target || !targetId || targetId === dragging) return;
  /* Only swap past the target's midpoint, so tabs of different widths don't flip back and forth. */
  const rect = target.getBoundingClientRect();
  const midpoint = rect.left + rect.width / 2;
  const ids = tabs.value.map((tab) => tab.id);
  const movingRight = ids.indexOf(targetId) > ids.indexOf(dragging);
  if (movingRight ? event.clientX > midpoint : event.clientX < midpoint) moveTab(dragging, targetId);
}

function endReorder() {
  const wasDragging = draggingId.value;
  pressed = null;
  draggingId.value = null;
  window.removeEventListener("pointermove", onReorderMove);
  window.removeEventListener("pointerup", endReorder);
  window.removeEventListener("pointercancel", endReorder);
  if (wasDragging) activate(wasDragging);
}

/*
 * The tab row never shows a scrollbar, so the active tab scrolls itself into
 * view (a new tab, or one picked by hotkey or menu), and a mouse wheel's
 * up and down scrolls the row sideways. Trackpads scroll it sideways already.
 */
watch(
  () => [activeId.value, tabs.value.length] as const,
  async () => {
    await nextTick();
    tabList.value?.querySelector<HTMLElement>(".tab.active")?.scrollIntoView({ block: "nearest", inline: "nearest" });
    updateOverflow();
  },
);

/* Each edge fades out while there are more tabs past it. */
const moreLeft = ref(false);
const moreRight = ref(false);
function updateOverflow() {
  const list = tabList.value;
  if (!list) return;
  moreLeft.value = list.scrollLeft > 1;
  moreRight.value = list.scrollLeft + list.clientWidth < list.scrollWidth - 1;
}
/* Watches the row and each tab, so a tab growing or shrinking counts too. */
const overflowObserver = new ResizeObserver(updateOverflow);
const tabMutations = new MutationObserver(() => {
  overflowObserver.disconnect();
  observeTabs();
});
function observeTabs() {
  const list = tabList.value;
  if (!list) return;
  overflowObserver.observe(list);
  for (const tab of Array.from(list.children)) overflowObserver.observe(tab);
  updateOverflow();
}

function onTabListWheel(event: WheelEvent) {
  const list = tabList.value;
  if (!list || event.deltaX !== 0 || event.deltaY === 0 || list.scrollWidth <= list.clientWidth) return;
  event.preventDefault();
  list.scrollLeft += event.deltaY;
}

/** Middle-click closes a tab, as in a browser. */
function onAuxClick(event: MouseEvent, id: string) {
  if (event.button === 1) closeTab(id);
}

/*
 * Renaming in place: double-click a terminal tab (or Rename… in its menu) and
 * its title becomes a field. Enter or clicking away keeps the name, Escape
 * drops it, and an empty name goes back to the shell's title.
 */
const renameDraft = ref("");
/** The tab's width when renaming began, held so swapping the title for a field can't resize it. */
const renameWidth = ref(0);
const renameInput = ref<HTMLInputElement | null>(null);
/* A function ref: a plain ref inside v-for would collect an array. */
const setRenameInput = (el: unknown) => {
  renameInput.value = el instanceof HTMLInputElement ? el : null;
};

function startRename(id: string) {
  if (!isReorderable(id)) return;
  const element = tabList.value?.querySelector<HTMLElement>(`[data-reorder-id="${id}"]`);
  renameWidth.value = element?.getBoundingClientRect().width ?? 0;
  /* Not activate(), which would hand the keyboard back to the terminal over the field. */
  activeId.value = id;
  renameDraft.value = tabName(id) || (tabs.value.find((tab) => tab.id === id)?.title ?? "");
  renamingTab.value = id;
}

watch(renamingTab, async (id) => {
  if (!id) return;
  if (!renameDraft.value) renameDraft.value = tabName(id) || (tabs.value.find((tab) => tab.id === id)?.title ?? "");
  await nextTick();
  renameInput.value?.focus();
  renameInput.value?.select();
});

function finishRename(save: boolean) {
  const id = renamingTab.value;
  if (!id) return;
  if (save) renameTab(id, renameDraft.value);
  renamingTab.value = null;
  renameDraft.value = "";
  activate(id);
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

function onTabContextMenu(event: MouseEvent, tab: AppTab) {
  if (tab.kind !== "terminal") {
    openContextMenu(event, [{ kind: "item", label: "Close Tab", action: () => closeTab(tab.id) }]);
    return;
  }
  openContextMenu(event, [
    { kind: "item", label: "Rename Tab…", action: () => startRename(tab.id) },
    { kind: "colors", label: "Tab Color", value: tab.color, pick: (color) => setTabColor(tab.id, color) },
    { kind: "separator" },
    { kind: "item", label: "Close Tab", danger: true, action: () => closeTab(tab.id) },
  ]);
}

/*
 * On macOS the window's own close/minimize/zoom buttons sit over the left of
 * the tab bar (an overlay title bar), so the bar leaves room for them, except
 * in full screen, where macOS hides them.
 */
const fullscreen = ref(false);
let stopResize: (() => void) | undefined;

async function syncFullscreen() {
  fullscreen.value = await getCurrentWindow()
    .isFullscreen()
    .catch(() => false);
}

onMounted(async () => {
  observeTabs();
  if (tabList.value) tabMutations.observe(tabList.value, { childList: true });
  if (!isMac) return;
  await syncFullscreen();
  stopResize = await getCurrentWindow().onResized(() => void syncFullscreen());
});

onBeforeUnmount(() => {
  overflowObserver.disconnect();
  tabMutations.disconnect();
  endReorder();
  stopResize?.();
});
</script>

<template>
  <div class="tab-bar" role="tablist" aria-label="Open tabs" data-tauri-drag-region>
    <div v-if="isMac && !fullscreen" class="window-controls-space" data-tauri-drag-region />
    <div
      ref="tabList"
      class="tab-list"
      :class="{ reordering: draggingId, 'more-left': moreLeft, 'more-right': moreRight }"
      data-tauri-drag-region
      @wheel="onTabListWheel"
      @scroll.passive="updateOverflow"
    >
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="tab"
        :class="{ active: activeId === tab.id, dragging: draggingId === tab.id, colored: tab.color, renaming: renamingTab === tab.id }"
        :style="[
          tab.color ? { '--tab-rgb': colorRgb(tab.color) } : {},
          renamingTab === tab.id && renameWidth
            ? { width: `${renameWidth}px`, minWidth: `${renameWidth}px`, maxWidth: `${renameWidth}px` }
            : {},
        ]"
        :data-reorder-id="isReorderable(tab.id) ? tab.id : undefined"
        :title="tab.title"
        role="tab"
        :aria-selected="activeId === tab.id"
        @click="activate(tab.id)"
        @auxclick="onAuxClick($event, tab.id)"
        @pointerdown="startReorder($event, tab.id)"
        @dblclick="startRename(tab.id)"
        @contextmenu="onTabContextMenu($event, tab)"
      >
        <svg v-if="tab.kind === 'settings'" class="tab-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.28Z"
          />
          <path d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
        </svg>
        <svg v-else-if="tab.kind === 'changelog'" class="tab-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z"
          />
        </svg>
        <svg v-else class="tab-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="m5.5 8 4 4-4 4" />
          <path d="M12 16.5h6.5" />
        </svg>
        <input
          v-if="renamingTab === tab.id"
          :ref="setRenameInput"
          v-model="renameDraft"
          class="tab-rename"
          type="text"
          spellcheck="false"
          autocomplete="off"
          :aria-label="`Rename ${tab.title}`"
          @keydown.stop="onRenameKeydown"
          @blur="finishRename(true)"
          @dblclick.stop
        />
        <span v-else class="tab-title"><NameLabel :text="tab.title" :badges="tab.named" /></span>
        <button class="tab-close" type="button" :aria-label="`Close ${tab.title}`" @click.stop="closeTab(tab.id)">
          <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path
              d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.75.75 0 1 1 1.06 1.06L9.06 8l3.22 3.22a.75.75 0 1 1-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 0 1-1.06-1.06L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z"
            />
          </svg>
        </button>
      </div>
      <button class="tab-new" type="button" :title="newTabTitle" aria-label="New tab" @click="newTerminal()">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 5v14" />
          <path d="M5 12h14" />
        </svg>
      </button>
    </div>
    <div class="tab-tools">
      <button
        v-if="updateReady"
        class="tab-tool update"
        type="button"
        :title="availableVersion ? `Update to v${availableVersion}` : 'Update available'"
        :aria-label="availableVersion ? `Update to v${availableVersion}` : 'Update available'"
        @click="showPrompt"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 4v10" />
          <path d="m8 10 4 4 4-4" />
          <path d="M5 18h14" />
        </svg>
      </button>
      <LayoutMenu />
      <button
        class="tab-version"
        :class="{ active: activeId === CHANGELOG_TAB_ID }"
        type="button"
        title="Open the change log"
        @click="openChangelog"
      >
        <span class="tab-version-prefix">v</span>{{ appVersion }}
      </button>
      <button
        class="tab-tool"
        type="button"
        title="Open Dispatch on GitHub"
        aria-label="Open Dispatch on GitHub"
        @click="openGitHub"
      >
        <svg class="tab-tool-github" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8"
          />
        </svg>
      </button>
      <button
        class="tab-tool"
        :class="{ active: activeId === SETTINGS_TAB_ID }"
        type="button"
        title="Settings"
        aria-label="Settings"
        @click="openSettings()"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.28Z"
          />
          <path d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
        </svg>
      </button>
    </div>
  </div>
</template>
