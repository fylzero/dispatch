<script setup lang="ts">
import { confirm } from "@tauri-apps/plugin-dialog";
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { aliasStartupFiles, errorMessage, setAliasesEverywhere, type StartupFile } from "../../api";
import { settings, updateSettings } from "../../settings";
import type { Alias, AliasGroup } from "../../types";

/*
 * Use in other terminals: Dispatch writes the aliases to ~/.config/dispatch
 * and adds a marked block to each startup file that loads them. The switch
 * shows what the files say, so a block removed by hand shows as off.
 */
const startupFiles = ref<StartupFile[] | null>(null);
const everywhereBusy = ref(false);
const everywhereError = ref("");
const everywhereChanged = ref(false);
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
  everywhereChanged.value = true;
  void runEverywhere(() => setAliasesEverywhere(enabled));
}

function fileState(file: StartupFile) {
  if (file.error) return "Couldn't change it. Add this line yourself:";
  if (file.state === "added") return "Loads these aliases.";
  if (file.state === "yours") return "Loads these aliases with a line of your own, which Dispatch leaves alone.";
  return everywhere.value ? "Not set up yet." : "Not changed.";
}

onMounted(() => void runEverywhere(aliasStartupFiles));

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
          <p class="muted tiny">
            Type a name to run its command; anything after the name goes after the command. Defined in zsh, bash,
            PowerShell and cmd after your own startup files, so one here replaces an alias of the same name, in tabs
            opened after a change. Groups just keep them in order.
          </p>
        </div>
        <button type="button" class="alias-add" @click="addGroup">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14" /><path d="M5 12h14" /></svg>
          Add group
        </button>
      </div>

      <section class="settings-card">
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Use in other terminals</h3>
            <p class="muted tiny">
              Load these aliases in Terminal, iTerm, VS Code and any other terminal too. Dispatch keeps them in
              ~/.config/dispatch and adds a marked block that loads them to the top and end of your startup files, so
              the rest of the file can use them and they still win over one of the same name. Turning this off takes
              out only those blocks.
            </p>
          </div>
          <div class="settings-control">
            <button
              class="switch"
              :class="{ on: everywhere }"
              type="button"
              role="switch"
              :aria-checked="everywhere"
              aria-label="Use in other terminals"
              :disabled="everywhereBusy || startupFiles === null || (!everywhere && !startupFiles.length)"
              @click="setEverywhere(!everywhere)"
            >
              <span class="switch-track" aria-hidden="true"><span class="switch-knob" /></span>
            </button>
          </div>
        </div>
        <p v-if="everywhereError" class="muted tiny settings-error alias-files-note">{{ everywhereError }}</p>
        <p v-else-if="startupFiles && !startupFiles.length" class="muted tiny alias-files-note">
          No zsh, bash or PowerShell startup files found.
        </p>
        <ul v-else-if="startupFiles" class="alias-files">
          <li v-for="file in startupFiles" :key="file.path">
            <div class="alias-file-head">
              <span class="alias-file-shell">{{ file.shell }}</span>
              <code>{{ file.path }}</code>
            </div>
            <p class="muted tiny" :class="{ 'settings-error': file.error }">
              {{ fileState(file) }}
              <template v-if="file.writesTo && !file.error"> It's a link, so the change goes in {{ file.writesTo }}.</template>
            </p>
            <template v-if="file.error">
              <code class="alias-file-line">{{ file.line }}</code>
              <p class="muted tiny">{{ file.error }}</p>
            </template>
          </li>
        </ul>
        <div v-if="missingFiles" class="alias-files-note">
          <button type="button" class="ghost" :disabled="everywhereBusy" @click="setEverywhere(true)">Set up the rest</button>
        </div>
        <p v-if="everywhereChanged && everywhere && !everywhereError" class="muted tiny alias-files-note">
          Terminals that are already open load them in their next new tab or window.
        </p>
      </section>

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

.alias-files {
  list-style: none;
  margin: 0;
  padding: 0 0 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.alias-files li p {
  margin: 0.15rem 0 0;
}

.alias-file-head {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  min-width: 0;
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
