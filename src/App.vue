<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, watch } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import TabBar from "./components/TabBar.vue";
import TerminalTabView from "./components/TerminalTabView.vue";
import ContextMenu from "./components/ContextMenu.vue";
import Modal from "./components/Modal.vue";
import Toast from "./components/Toast.vue";
import { useUpdater } from "./composables/useUpdater";
import { isMac } from "./platform";
import { applySavedSettings, loadSettings, settings, settingsLoaded, zoom } from "./settings";
import {
  CHANGELOG_TAB_ID,
  SETTINGS_TAB_ID,
  activateIndex,
  activeId,
  activeTerminal,
  changelogOpen,
  closeActive,
  cycleTab,
  dragOver,
  dropPaths,
  focusNeighbor,
  newTerminal,
  openSettings,
  openStartTabs,
  settingsOpen,
  splitActive,
  tabs,
  terminalTabs,
} from "./tabs";
import type { Settings } from "./types";
import { dismissToast, pauseToast, resumeToast, showToast, toastKind, toastMessage } from "./toast";

const SettingsView = defineAsyncComponent(() => import("./views/SettingsView.vue"));
const ChangelogView = defineAsyncComponent(() => import("./views/ChangelogView.vue"));

const unlisten: UnlistenFn[] = [];

const {
  status: updateStatus,
  statusText: updateStatusText,
  currentVersion,
  availableVersion,
  busy: updateBusy,
  promptOpen,
  checkForUpdates,
  installUpdate,
  dismissPrompt,
  startAutoChecks,
} = useUpdater();

/** Check for Updates… from the menu: always say how it went. */
async function checkForUpdatesNow() {
  await checkForUpdates({ prompt: true });
  if (updateStatus.value === "up-to-date") showToast("You're on the latest version.");
  else if (updateStatus.value === "error") showToast(updateStatusText.value, "error");
}

/** The window title follows the active tab, like other terminals. */
const activeTitle = computed(() => tabs.value.find((tab) => tab.id === activeId.value)?.title ?? "");
watch(activeTitle, (title) => {
  void getCurrentWindow()
    .setTitle(title ? `${title} — Dispatch` : "Dispatch")
    .catch(() => undefined);
});

/** ⌘1–⌘8 pick a tab and ⌘9 the last one (Alt+digit elsewhere, where Ctrl+digit is the shell's). */
function onWindowKeydown(event: KeyboardEvent) {
  const modifier = isMac ? event.metaKey && !event.ctrlKey : event.altKey && !event.ctrlKey && !event.metaKey;
  if (!modifier || event.shiftKey || !/^[1-9]$/.test(event.key)) return;
  event.preventDefault();
  event.stopPropagation();
  const digit = Number(event.key);
  activateIndex(digit === 9 ? -1 : digit - 1);
}

function onContextMenu(event: MouseEvent) {
  /* The webview's default menu (Reload, Inspect) only gets in the way. */
  const target = event.target as HTMLElement | null;
  if (!target?.closest("input, textarea:not(.xterm-helper-textarea)")) event.preventDefault();
}

onMounted(async () => {
  /* Menu shortcuts go to the window in front only, so listen on this window, not app-wide. */
  const appWindow = getCurrentWebviewWindow();
  const listen = appWindow.listen.bind(appWindow);
  window.addEventListener("keydown", onWindowKeydown, true);
  window.addEventListener("contextmenu", onContextMenu);
  unlisten.push(
    await listen("new-tab", () => newTerminal()),
    await listen("close-tab-or-window", () => closeActive()),
    await listen("split-vertical", () => splitActive("vertical")),
    await listen("split-horizontal", () => splitActive("horizontal")),
    await listen("focus-pane-left", () => focusNeighbor("left")),
    await listen("focus-pane-right", () => focusNeighbor("right")),
    await listen("focus-pane-up", () => focusNeighbor("up")),
    await listen("focus-pane-down", () => focusNeighbor("down")),
    await listen("next-tab", () => cycleTab(1)),
    await listen("previous-tab", () => cycleTab(-1)),
    await listen("open-settings", () => openSettings()),
    await listen("clear-terminal", () => activeTerminal()?.clear()),
    await listen("select-all", () => {
      /* ⌘A is a menu shortcut, so a text field (renaming a tab or pane) has to be selected by hand. */
      const field = document.activeElement;
      if (field instanceof HTMLInputElement && field.type === "text") field.select();
      else activeTerminal()?.selectAll();
    }),
    await listen("zoom-in", () => zoom(1)),
    await listen("zoom-out", () => zoom(-1)),
    await listen("zoom-reset", () => zoom("reset")),
    await listen("check-for-updates", () => void checkForUpdatesNow()),
    /* Saved in another window. */
    await listen<Settings>("settings-changed", ({ payload }) => applySavedSettings(payload)),
    /* Tauri takes file drops from the webview, so their paths come through here. */
    await getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "leave") return dragOver(-1, -1);
      const { x, y } = payload.position.toLogical(window.devicePixelRatio);
      if (payload.type === "drop") dropPaths(payload.paths, x, y);
      else dragOver(x, y);
    }),
  );
  await loadSettings();
  /* Only the first window, made when Dispatch starts, opens the layout set to open at launch. */
  openStartTabs(appWindow.label === "main");
  /* Look for a newer release now and every few hours; the tab bar shows an icon and a prompt if there is one. */
  unlisten.push(startAutoChecks(() => settings.value.autoCheckUpdates));
});

onUnmounted(() => {
  window.removeEventListener("keydown", onWindowKeydown, true);
  window.removeEventListener("contextmenu", onContextMenu);
  unlisten.forEach((fn) => fn());
});
</script>

<template>
  <div class="app-shell" :style="{ '--terminal-opacity': settings.opacity, '--pane-dim': settings.paneDim }">
    <TabBar />
    <main class="main">
      <template v-if="settingsLoaded">
        <div v-for="tab in terminalTabs" v-show="activeId === tab.id" :key="tab.id" class="main-pane terminal-main">
          <TerminalTabView :tab="tab" :visible="activeId === tab.id" />
        </div>
      </template>
      <div v-if="settingsOpen" v-show="activeId === SETTINGS_TAB_ID" class="main-pane">
        <SettingsView />
      </div>
      <div v-if="changelogOpen" v-show="activeId === CHANGELOG_TAB_ID" class="main-pane">
        <ChangelogView />
      </div>
    </main>
    <Modal v-if="promptOpen" title="Update available" @close="updateBusy ? undefined : dismissPrompt()">
      <p>Dispatch {{ availableVersion }} is available. You have {{ currentVersion }}.</p>
      <p
        v-if="updateStatus === 'downloading' || updateStatus === 'installing' || updateStatus === 'error'"
        class="muted tiny"
        :class="{ 'settings-error': updateStatus === 'error' }"
      >
        {{ updateStatusText }}
      </p>
      <template #actions>
        <button class="ghost" type="button" :disabled="updateBusy" @click="dismissPrompt">Later</button>
        <button class="primary" type="button" :disabled="updateBusy" @click="installUpdate">Install and restart</button>
      </template>
    </Modal>
    <ContextMenu />
    <Transition name="toast" :duration="{ enter: 520, leave: 280 }">
      <Toast
        v-if="toastMessage"
        :message="toastMessage"
        :kind="toastKind"
        @dismiss="dismissToast"
        @pause="pauseToast"
        @resume="resumeToast"
      />
    </Transition>
  </div>
</template>
