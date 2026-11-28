<script setup lang="ts">
import { confirm } from "@tauri-apps/plugin-dialog";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { aliasStartupFiles, errorMessage, setAliasesEverywhere, type StartupFile } from "../../api";
import { placePopover, useDismiss, type PopoverPosition } from "../../composables/usePopover";
import { settings, updateSettings } from "../../settings";
import type { Alias, AliasGroup } from "../../types";

/*
 * Make aliases global: Dispatch writes the aliases to ~/.config/dispatch
 * and adds a marked block to each startup file that loads them. The switch
 * shows what the files say, so a block removed by hand shows as off.
 */
const startupFiles = ref<StartupFile[] | null>(null);
const everywhereBusy = ref(false);
const everywhereError = ref("");
const everywhere = computed(() => startupFiles.value?.some((file) => file.state === "added") ?? false);
const missingFiles = computed(
  () => everywhere.value && (startupFiles.value?.some((file) => file.state === "off" && !file.error) ?? false),
);

async function runEverywhere(task: () => Promise<StartupFile[]>) {
  everywhereBusy.value = true;
  everywhereError.value = "";
  try {
    startupFiles.value = await task();
  } catch (err) {
    everywhereError.value = errorMessage(err);
  } finally {
    everywhereBusy.value = false;
  }
}

function setEverywhere(enabled: boolean) {
  void runEverywhere(() => setAliasesEverywhere(enabled));
}

/** Short status shown as a pill beside each file. */
function fileStatus(file: StartupFile): { label: string; tone: "good" | "info" | "bad" | "idle" } {
  if (file.error) return { label: "Couldn't change", tone: "bad" };
  if (file.state === "added") return { label: "Active", tone: "good" };
  if (file.state === "yours") return { label: "Active, your line", tone: "info" };
  return { label: everywhere.value ? "Not set up" : "Off", tone: "idle" };
}

/** Extra detail under a file, only when there's something worth saying. */
function fileNote(file: StartupFile) {
  if (file.error) return "Add this line to the file yourself:";
  if (file.state === "yours") return "This file already loads them with a line of your own, which Dispatch leaves alone.";
  if (file.writesTo) return `It's a link, so the change goes in ${file.writesTo}.`;
  return "";
}

onMounted(() => void runEverywhere(aliasStartupFiles));

/* What the switch does and each file's state live in a popover from the info button; problems stay on the card. */
const failedFiles = computed(() => startupFiles.value?.filter((file) => file.error) ?? []);
const infoOpen = ref(false);
const infoButton = ref<HTMLElement | null>(null);
const infoPopover = ref<HTMLElement | null>(null);
const infoPosition = ref<PopoverPosition>({ left: 0, top: 0 });

function toggleInfo() {
  infoOpen.value = !infoOpen.value;
  if (!infoOpen.value) return;
  void nextTick(() => {
    if (infoButton.value) infoPosition.value = placePopover(infoButton.value.getBoundingClientRect(), infoPopover.value);
  });
}

useDismiss(infoOpen, () => [infoPopover.value, infoButton.value], () => (infoOpen.value = false));

/* It's placed against the window, so close it when the page scrolls out from under it. */
function closeInfoOnScroll(event: Event) {
  if (!(event.target instanceof Node && infoPopover.value?.contains(event.target))) infoOpen.value = false;
}
watch(infoOpen, (open) =>
  open
    ? window.addEventListener("scroll", closeInfoOnScroll, true)
    : window.removeEventListener("scroll", closeInfoOnScroll, true),
);
onBeforeUnmount(() => window.removeEventListener("scroll", closeInfoOnScroll, true));

/* Matches valid_name in aliases.rs: a plain word in every shell. */
const ALIAS_NAME = /^[A-Za-z0-9_.][A-Za-z0-9_.-]*$/;
const DRAG_THRESHOLD = 4;

const page = ref<HTMLElement | null>(null);
const groups = computed(() => settings.value.aliasGroups);

function setGroups(next: AliasGroup[]) {
  updateSettings({ aliasGroups: next });
}

/** Why each row won't be defined, or null when it will be. Blank rows say nothing. */
const problems = computed(() => {
  /* The shells see one list, group after group, and the last of a name wins. */
  const last = new Map<string, string>();
  groups.value.forEach((group, g) =>
    group.aliases.forEach((alias, i) => last.set(alias.name.trim(), `${g}:${i}`)),
  );
  return groups.value.map((group, g) =>
    group.aliases.map((alias, i) => {
      const name = alias.name.trim();
      const command = alias.command.trim();
      if (!name && !command) return null;
      if (!name) return "Give it a name.";
      if (!ALIAS_NAME.test(name)) return "Names use letters, digits, _ - and . only, and can't start with -.";
      if (!command) return "Give it a command.";
      if (last.get(name) !== `${g}:${i}`) return "Used again further down, which wins.";
      return null;
    }),
  );
});

function setGroupName(g: number, name: string) {
  setGroups(groups.value.map((group, i) => (i === g ? { ...group, name } : group)));
}

function setAlias(g: number, i: number, patch: Partial<Alias>) {
  setGroups(
    groups.value.map((group, gi) =>
      gi === g
        ? { ...group, aliases: group.aliases.map((alias, ai) => (ai === i ? { ...alias, ...patch } : alias)) }
        : group,
    ),
  );
}

async function focusLast(selector: string) {
  await nextTick();
  const found = page.value?.querySelectorAll<HTMLInputElement>(selector);
  found?.[found.length - 1]?.focus();
}

function addGroup() {
  setGroups([...groups.value, { name: "", aliases: [{ name: "", command: "" }] }]);
  void focusLast(".alias-group-name");
}

function addAlias(g: number) {
  setGroups(
    groups.value.map((group, i) =>
      i === g ? { ...group, aliases: [...group.aliases, { name: "", command: "" }] } : group,
    ),
  );
  void focusLast(`[data-group="${g}"] .alias-name`);
}

function removeAlias(g: number, i: number) {
  setGroups(
    groups.value.map((group, gi) =>
      gi === g ? { ...group, aliases: group.aliases.filter((_, ai) => ai !== i) } : group,
    ),
  );
}

async function removeGroup(g: number) {
  const group = groups.value[g];
  const filled = group.aliases.filter((alias) => alias.name.trim() || alias.command.trim()).length;
  if (filled) {
    const title = group.name.trim() ? `“${group.name.trim()}”` : "this group";
    const ok = await confirm(`Remove ${title} and its ${filled === 1 ? "alias" : `${filled} aliases`}?`, {
      title: "Remove group",
      kind: "warning",
      okLabel: "Remove",
    });
    if (!ok) return;
  }
  setGroups(groups.value.filter((_, i) => i !== g));
}

function groupLabel(group: AliasGroup) {
  return group.name.trim() || "Untitled group";
}

/*
 * Dragging, with pointer events as in the tab bar (HTML drag and drop is
 * unreliable in the app's web view). Grab an alias by its handle to move it
 * within its group or into another one, or a group by its handle to reorder
 * the groups. The list changes as the pointer moves.
 */
type Drag = { kind: "alias"; g: number; i: number } | { kind: "group"; g: number };
const drag = ref<Drag | null>(null);
let pressed: { drag: Drag; y: number } | null = null;

function startDrag(event: PointerEvent, target: Drag) {
  if (event.button !== 0) return;
  event.preventDefault();
  pressed = { drag: target, y: event.clientY };
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", endDrag);
  window.addEventListener("pointercancel", endDrag);
}

/** How many of `elements` (top to bottom) sit above `y`, by their middles. */
function slotAt(elements: HTMLElement[], y: number) {
  return elements.filter((element) => {
    const rect = element.getBoundingClientRect();
    return rect.top + rect.height / 2 < y;
  }).length;
}

function onDragMove(event: PointerEvent) {
  if (!pressed) return;
  if (!drag.value) {
    if (Math.abs(event.clientY - pressed.y) < DRAG_THRESHOLD) return;
    drag.value = pressed.drag;
  }
  const current = drag.value;
  const groupElements = Array.from(page.value?.querySelectorAll<HTMLElement>(".alias-group") ?? []);
  if (current.kind === "group") {
    const others = groupElements.filter((_, g) => g !== current.g);
    moveGroup(current.g, slotAt(others, event.clientY));
    return;
  }
  /* The group under the pointer, or the nearest one above or below the list. */
  let target = groupElements.findIndex((element) => {
    const rect = element.getBoundingClientRect();
    return event.clientY >= rect.top && event.clientY < rect.bottom;
  });
  if (target < 0) {
    if (!groupElements.length) return;
    target = event.clientY < groupElements[0].getBoundingClientRect().top ? 0 : groupElements.length - 1;
  }
  const rows = Array.from(groupElements[target].querySelectorAll<HTMLElement>(".alias-row")).filter(
    (_, i) => !(target === current.g && i === current.i),
  );
  moveAlias(current.g, current.i, target, slotAt(rows, event.clientY));
}

function endDrag() {
  pressed = null;
  drag.value = null;
  window.removeEventListener("pointermove", onDragMove);
  window.removeEventListener("pointerup", endDrag);
  window.removeEventListener("pointercancel", endDrag);
}

onBeforeUnmount(endDrag);

function moveGroup(from: number, to: number) {
  if (from === to || to < 0 || to >= groups.value.length) return;
  const next = [...groups.value];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  setGroups(next);
  if (drag.value?.kind === "group") drag.value = { kind: "group", g: to };
}

function moveAlias(fromGroup: number, from: number, toGroup: number, to: number) {
  if (fromGroup === toGroup && from === to) return;
  const next = groups.value.map((group) => ({ ...group, aliases: [...group.aliases] }));
  const [moved] = next[fromGroup].aliases.splice(from, 1);
  next[toGroup].aliases.splice(to, 0, moved);
  setGroups(next);
  if (drag.value?.kind === "alias") drag.value = { kind: "alias", g: toGroup, i: to };
}

/* The handles also move with the arrow keys, an alias across into the next group at either end. */
async function keyMoveAlias(event: KeyboardEvent, g: number, i: number) {
  const up = event.key === "ArrowUp";
  if (!up && event.key !== "ArrowDown") return;
  event.preventDefault();
  const count = groups.value[g].aliases.length;
  let toGroup = g;
  let to = up ? i - 1 : i + 1;
  if (to < 0) {
    if (g === 0) return;
    toGroup = g - 1;
    to = groups.value[toGroup].aliases.length;
  } else if (to >= count) {
    if (g === groups.value.length - 1) return;
    toGroup = g + 1;
    to = 0;
  }
  moveAlias(g, i, toGroup, to);
  await nextTick();
  page.value
    ?.querySelectorAll<HTMLElement>(`[data-group="${toGroup}"] .alias-row .drag-handle`)
    [to]?.focus();
}

async function keyMoveGroup(event: KeyboardEvent, g: number) {
  const up = event.key === "ArrowUp";
  if (!up && event.key !== "ArrowDown") return;
  event.preventDefault();
  const to = up ? g - 1 : g + 1;
  if (to < 0 || to >= groups.value.length) return;
  moveGroup(g, to);
  await nextTick();
  page.value?.querySelectorAll<HTMLElement>(".alias-group-head .drag-handle")[to]?.focus();
}
</script>

<template>
  <div ref="page" class="settings-pane settings-form-page" :class="{ 'alias-dragging': drag }">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">Aliases</div>
        </div>
        <button type="button" class="alias-add" @click="addGroup">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14" /><path d="M5 12h14" /></svg>
          Add group
        </button>
      </div>

      <section class="settings-card">
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3 class="alias-global-title">
              Make aliases global
              <button
                ref="infoButton"
                type="button"
                class="alias-info"
                :class="{ open: infoOpen }"
                aria-label="About making aliases global"
                :aria-expanded="infoOpen"
                @click="toggleInfo"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <circle cx="12" cy="12" r="9" /><path d="M12 11v5" /><path d="M12 8h.01" />
                </svg>
              </button>
            </h3>
          </div>
          <button
            class="switch"
            :class="{ on: everywhere }"
            type="button"
            role="switch"
            :aria-checked="everywhere"
            aria-label="Make aliases global"
            :disabled="everywhereBusy || startupFiles === null || (!everywhere && !startupFiles.length)"
            @click="setEverywhere(!everywhere)"
          >
            <span class="switch-track" aria-hidden="true">
              <span class="switch-knob" />
            </span>
            {{ everywhere ? "On" : "Off" }}
          </button>
        </div>
        <!-- Only what needs you stays on the card. -->
        <p v-if="everywhereError" class="muted tiny settings-error alias-files-note">{{ everywhereError }}</p>
        <p v-else-if="startupFiles && !startupFiles.length" class="muted tiny alias-files-note">
          No zsh, bash or PowerShell startup files found.
        </p>
        <ul v-if="failedFiles.length" class="alias-files">
          <li v-for="file in failedFiles" :key="file.path">
            <div class="alias-file-head">
              <span class="alias-file-shell">{{ file.shell }}</span>
              <code>{{ file.path }}</code>
              <span class="alias-file-status bad">Couldn't change</span>
            </div>
            <p class="muted tiny settings-error">Add this line to the file yourself:</p>
            <code class="alias-file-line">{{ file.line }}</code>
            <p class="muted tiny">{{ file.error }}</p>
          </li>
        </ul>
        <div v-if="missingFiles" class="alias-files-note">
          <button type="button" class="ghost" :disabled="everywhereBusy" @click="setEverywhere(true)">Set up the rest</button>
        </div>
      </section>

      <Teleport to="body">
        <div
          v-if="infoOpen"
          ref="infoPopover"
          class="alias-info-popover"
          role="dialog"
          aria-label="About making aliases global"
          :style="{ left: `${infoPosition.left}px`, top: `${infoPosition.top}px` }"
        >
          <div class="alias-info-states">
            <div class="alias-info-state" :class="{ current: !everywhere }">
              <span class="alias-info-label">Off</span>
              <span>Your aliases work only in Dispatch.</span>
            </div>
            <div class="alias-info-state" :class="{ current: everywhere }">
              <span class="alias-info-label on">On</span>
              <span>They work in every terminal on this computer.</span>
            </div>
          </div>
          <h4 v-if="startupFiles?.length">Startup files</h4>
          <ul v-if="startupFiles?.length" class="alias-files">
            <li v-for="file in startupFiles" :key="file.path">
              <div class="alias-file-head">
                <span class="alias-file-shell">{{ file.shell }}</span>
                <code>{{ file.path }}</code>
                <span class="alias-file-status" :class="fileStatus(file).tone">{{ fileStatus(file).label }}</span>
              </div>
              <p v-if="fileNote(file) && !file.error" class="muted tiny">{{ fileNote(file) }}</p>
            </li>
          </ul>
          <h4>How it works</h4>
          <p class="muted tiny alias-files-how">
            Dispatch saves your aliases in <code>~/.config/dispatch</code> and adds a small marked block to the top and
            bottom of each file, so the rest of the file can use them. Turning this off removes only that block.
            Terminals that are already open pick them up in their next new tab or window.
          </p>
        </div>
      </Teleport>

      <section v-if="!groups.length" class="settings-card alias-empty">
        <p class="muted tiny">No aliases yet. Add a group to start, like “git” or “deploy”.</p>
      </section>

      <section
        v-for="(group, g) in groups"
        :key="g"
        class="settings-card alias-group"
        :class="{ dragging: drag?.kind === 'group' && drag.g === g }"
        :data-group="g"
      >
        <div class="alias-group-head">
          <button
            type="button"
            class="ghost drag-handle"
            :aria-label="`Move group ${groupLabel(group)}`"
            title="Drag to reorder"
            @pointerdown="startDrag($event, { kind: 'group', g })"
            @keydown="keyMoveGroup($event, g)"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="9" cy="6" r="1" /><circle cx="15" cy="6" r="1" /><circle cx="9" cy="12" r="1" />
              <circle cx="15" cy="12" r="1" /><circle cx="9" cy="18" r="1" /><circle cx="15" cy="18" r="1" />
            </svg>
          </button>
          <input
            class="alias-group-name"
            type="text"
            :value="group.name"
            placeholder="Untitled group"
            aria-label="Group name"
            autocapitalize="off"
            autocorrect="off"
            autocomplete="off"
            spellcheck="false"
            @input="setGroupName(g, ($event.target as HTMLInputElement).value)"
          />
          <button type="button" class="ghost alias-add" :aria-label="`Add alias to ${groupLabel(group)}`" @click="addAlias(g)">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14" /><path d="M5 12h14" /></svg>
            Add alias
          </button>
          <button
            type="button"
            class="ghost danger alias-icon"
            :aria-label="`Remove group ${groupLabel(group)}`"
            title="Remove group"
            @click="removeGroup(g)"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 7h16" /><path d="M10 11v6" /><path d="M14 11v6" />
              <path d="M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12" /><path d="M9 7V4h6v3" />
            </svg>
          </button>
        </div>

        <div class="alias-list">
          <div
            v-for="(alias, i) in group.aliases"
            :key="i"
            class="alias-row"
            :class="{ dragging: drag?.kind === 'alias' && drag.g === g && drag.i === i }"
          >
            <button
              type="button"
              class="ghost drag-handle"
              :aria-label="alias.name.trim() ? `Move alias ${alias.name.trim()}` : 'Move alias'"
              title="Drag to move"
              @pointerdown="startDrag($event, { kind: 'alias', g, i })"
              @keydown="keyMoveAlias($event, g, i)"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="9" cy="6" r="1" /><circle cx="15" cy="6" r="1" /><circle cx="9" cy="12" r="1" />
                <circle cx="15" cy="12" r="1" /><circle cx="9" cy="18" r="1" /><circle cx="15" cy="18" r="1" />
              </svg>
            </button>
            <input
              class="alias-name"
              type="text"
              :value="alias.name"
              placeholder="gs"
              aria-label="Alias name"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck="false"
              :aria-invalid="problems[g]?.[i] ? 'true' : undefined"
              @input="setAlias(g, i, { name: ($event.target as HTMLInputElement).value })"
            />
            <input
              class="alias-command"
              type="text"
              :value="alias.command"
              placeholder="git status"
              aria-label="Command"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck="false"
              @input="setAlias(g, i, { command: ($event.target as HTMLInputElement).value })"
            />
            <button
              type="button"
              class="ghost alias-icon"
              :aria-label="alias.name.trim() ? `Remove alias ${alias.name.trim()}` : 'Remove alias'"
              title="Remove"
              @click="removeAlias(g, i)"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>
            </button>
            <p v-if="problems[g]?.[i]" class="muted tiny settings-error alias-problem">{{ problems[g][i] }}</p>
          </div>
          <p v-if="!group.aliases.length" class="muted tiny alias-drop">Empty. Drag an alias here or add one.</p>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-header {
  gap: 1rem;
}

.alias-add {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  white-space: nowrap;
  flex: 0 0 auto;
}

svg {
  width: 14px;
  height: 14px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.drag-handle svg circle {
  fill: currentColor;
  stroke: none;
}

.alias-global-title {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}

/* A small circled i beside the title; opens what the switch does. */
.alias-info {
  display: inline-flex;
  padding: 0;
  border: 0;
  background: none;
  color: var(--muted);
}

button.alias-info:hover:not(:disabled),
.alias-info.open {
  background: none;
  color: var(--text);
}

.alias-info svg {
  width: 15px;
  height: 15px;
}

/* Recon's .filter-popover, with room for text. */
.alias-info-popover {
  position: fixed;
  z-index: 14;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  width: 26rem;
  max-width: calc(100vw - 8px);
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-raised);
  box-shadow: 0 12px 32px var(--shadow-strong);
  font-size: 13px;
}

.alias-info-popover p {
  margin: 0;
}

.alias-info-popover h4 {
  margin: 0.25rem 0 -0.2rem;
  color: var(--muted);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

/* Off and On side by side as two rows; the one in effect now stands out. */
.alias-info-states {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}

.alias-info-state {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.55rem 0.75rem;
  color: var(--muted);
}

.alias-info-state + .alias-info-state {
  border-top: 1px solid var(--border);
}

.alias-info-state.current {
  color: var(--text);
  background: var(--bg-hover);
}

.alias-info-label {
  flex: 0 0 2.6rem;
  padding: 0.1rem 0;
  border: 1px solid currentColor;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  text-align: center;
  color: var(--text-faint);
}

/* Only the badge for the setting in effect has color: red for Off, the accent for On. */
.alias-info-state.current .alias-info-label {
  color: var(--bad-text);
}

.alias-info-state.current .alias-info-label.on {
  color: var(--accent-text);
}

.alias-info-popover .alias-files {
  margin: 0;
}

.alias-files {
  list-style: none;
  margin: 0 0 0.75rem;
  padding: 0;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}

.alias-files li {
  padding: 0.55rem 0.75rem;
}

.alias-files li + li {
  border-top: 1px solid var(--border);
}

.alias-files li p {
  margin: 0.15rem 0 0;
}

.alias-file-head {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.alias-file-head code {
  flex: 1 1 auto;
  min-width: 0;
}

.alias-file-status {
  flex: 0 0 auto;
  font-size: 11px;
  font-weight: 600;
  padding: 0.1rem 0.5rem;
  border-radius: 999px;
  border: 1px solid currentColor;
  white-space: nowrap;
}

.alias-file-status.good {
  color: var(--good);
}

.alias-file-status.info {
  color: var(--accent-text);
}

.alias-file-status.bad {
  color: var(--bad-text);
}

.alias-file-status.idle {
  color: var(--text-faint);
}

.alias-file-shell {
  font-weight: 600;
  font-size: 12px;
}

.alias-files code {
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-size: 11px;
  overflow-wrap: anywhere;
}

.alias-file-line {
  display: block;
  margin-top: 0.3rem;
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  user-select: text;
}

.alias-files-note {
  margin: 0;
  padding-bottom: 0.75rem;
}

.alias-files-how code {
  font-family: var(--font-code);
  font-size: 11px;
}

.alias-empty {
  padding-bottom: 0.85rem;
}

.alias-empty p {
  margin: 0;
}

.alias-group {
  padding: 0.6rem 0.75rem 0.75rem;
  transition: box-shadow 120ms ease;
}

.alias-group.dragging,
.alias-row.dragging {
  box-shadow: 0 0 0 1px var(--accent);
  border-radius: 8px;
}

.alias-row.dragging {
  border-radius: 6px;
  background: var(--bg-raised);
}

.alias-group-head {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  margin-bottom: 0.5rem;
}

.alias-group-name {
  flex: 1;
  min-width: 0;
  font-weight: 600;
  background: transparent;
  border-color: transparent;
}

.alias-group-name:hover {
  border-color: var(--border);
}

.alias-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.alias-row {
  display: grid;
  grid-template-columns: auto minmax(5rem, 9rem) 1fr auto;
  gap: 0.4rem;
  align-items: center;
}

.alias-row input {
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-size: 12px;
}

.alias-name[aria-invalid="true"] {
  border-color: var(--bad);
}

.drag-handle,
.alias-icon {
  display: inline-flex;
  padding: 0.35rem;
  border-color: transparent;
}

.drag-handle {
  cursor: grab;
  touch-action: none;
  color: var(--muted);
  opacity: 0.7;
}

.drag-handle:hover {
  opacity: 1;
}

.alias-dragging,
.alias-dragging * {
  cursor: grabbing !important;
  user-select: none;
}

.alias-problem {
  grid-column: 2 / -1;
  margin: -0.15rem 0 0;
}

.alias-drop {
  margin: 0;
  padding: 0.6rem;
  border: 1px dashed var(--border);
  border-radius: 6px;
  text-align: center;
}
</style>
