<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import Modal from "./Modal.vue";
import { type MenuEntry, contextMenu, toggleDropdownMenu } from "../contextMenu";
import {
  MAX_LAYOUTS,
  MAX_LAYOUT_NAME,
  applyLayout,
  canRenameLayout,
  canSaveLayout,
  deleteLayout,
  editLayout,
  findLayout,
  isDefaultLayout,
  saveLayout,
  savedLayouts,
} from "../layouts";
import { currentLayout, defaultLayout, focusTerminalPane } from "../tabs";

const button = ref<HTMLButtonElement | null>(null);
const menuOpen = computed(() => !!button.value && contextMenu.value?.anchor === button.value);

/** The layout this window is using, if it is still saved. */
const current = computed(() => (currentLayout.value ? findLayout(currentLayout.value) : undefined));

function openMenu() {
  if (!button.value) return;
  const entries: MenuEntry[] = [{ kind: "item", label: "Save Layout…", action: () => openDialog(null) }];
  if (savedLayouts.value.length) {
    entries.push({ kind: "separator" });
    for (const saved of savedLayouts.value) {
      entries.push({
        kind: "item",
        label: saved.name,
        hint: isDefaultLayout(saved.name) ? "Opens at launch" : undefined,
        checked: current.value === saved,
        action: () => chooseLayout(saved.name),
        edit: { label: `Edit “${saved.name}”`, action: () => openDialog(saved.name) },
      });
    }
  }
  toggleDropdownMenu(button.value, entries);
}

/*
 * One dialog for Save Layout… and Edit “name”…: a name, which is required,
 * and whether Dispatch opens the layout at launch. Saving keeps this
 * window's tabs; editing only changes the name and the checkbox.
 */
const dialogOpen = ref(false);
/** The layout being edited, or null when saving. */
const editing = ref<string | null>(null);
const draftName = ref("");
const openAtLaunch = ref(false);
const nameInput = ref<HTMLInputElement | null>(null);
const trimmed = computed(() => draftName.value.trim());
const target = computed(() => (trimmed.value ? findLayout(trimmed.value) : undefined));
/** Saving over a layout with this name. */
const replacing = computed(() => (editing.value ? undefined : target.value));
const full = computed(() => !editing.value && !!trimmed.value && !canSaveLayout(trimmed.value));
const taken = computed(() => !!editing.value && !!trimmed.value && !canRenameLayout(editing.value, trimmed.value));
/** Another layout that opens at launch now, which ticking the box would take over from. */
const launchReplaced = computed(() => {
  const launch = defaultLayout();
  if (!openAtLaunch.value || !launch) return undefined;
  const self = editing.value ? findLayout(editing.value) : target.value;
  return self === launch ? undefined : launch;
});

function openDialog(edit: string | null) {
  editing.value = edit;
  draftName.value = edit ?? current.value?.name ?? "";
  const layout = edit ? findLayout(edit) : current.value;
  /* Ticked for the first layout saved, or for the one that opens at launch now. */
  openAtLaunch.value = (!edit && !savedLayouts.value.length) || (!!layout && isDefaultLayout(layout.name));
  dialogOpen.value = true;
  void nextTick(() => {
    nameInput.value?.focus();
    nameInput.value?.select();
  });
}

function closeDialog() {
  dialogOpen.value = false;
  editing.value = null;
  focusTerminalPane();
}

function removeEdited() {
  if (editing.value) deleteLayout(editing.value);
  closeDialog();
}

function submit() {
  if (!trimmed.value || full.value || taken.value) return;
  const done = editing.value
    ? editLayout(editing.value, trimmed.value, openAtLaunch.value)
    : saveLayout(trimmed.value, openAtLaunch.value);
  if (done) closeDialog();
}

/* Picking a layout makes this window match it: its tabs take the place of the open ones. */
function chooseLayout(name: string) {
  applyLayout(name, true);
}
</script>

<template>
  <button
    ref="button"
    class="tab-tool"
    :class="{ active: menuOpen }"
    type="button"
    title="Layouts"
    aria-label="Layouts"
    aria-haspopup="menu"
    :aria-expanded="menuOpen"
    @click="openMenu"
  >
    <svg class="tab-tool-dots" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="5.5" r="1.6" />
      <circle cx="12" cy="12" r="1.6" />
      <circle cx="12" cy="18.5" r="1.6" />
    </svg>
  </button>

  <Modal v-if="dialogOpen" :title="editing ? 'Edit layout' : 'Save layout'" @close="closeDialog">
    <p v-if="!editing" class="muted tiny">
      Saves this window's tabs and panes, with their names, colors and sizes. Pick it from the
      <span aria-label="layouts">⋮</span> menu to open it again.
    </p>
    <form class="layout-save-form" @submit.prevent="submit">
      <label class="modal-label">
        <span>Layout name</span>
        <input
          ref="nameInput"
          v-model="draftName"
          type="text"
          required
          :maxlength="MAX_LAYOUT_NAME"
          spellcheck="false"
          autocomplete="off"
          placeholder="e.g. Web project"
        />
      </label>
      <label class="layout-launch-check">
        <input v-model="openAtLaunch" type="checkbox" />
        <span>Open at launch</span>
      </label>
      <p v-if="launchReplaced" class="muted tiny layout-launch-note">
        Replaces “{{ launchReplaced.name }}”, which opens at launch now.
      </p>
    </form>
    <p v-if="full" class="muted tiny settings-error">
      Up to {{ MAX_LAYOUTS }} layouts can be saved. Delete one, or use the name of one to replace it.
    </p>
    <p v-else-if="taken" class="muted tiny settings-error">There's already a layout called “{{ trimmed }}”.</p>
    <p v-else-if="replacing" class="muted tiny">This replaces the saved layout “{{ replacing.name }}”.</p>
    <template #actions>
      <button v-if="editing" class="ghost danger layout-delete" type="button" @click="removeEdited">Delete</button>
      <button class="ghost" type="button" @click="closeDialog">Cancel</button>
      <button class="primary" type="button" :disabled="!trimmed || full || taken" @click="submit">
        {{ replacing ? "Replace" : "Save" }}
      </button>
    </template>
  </Modal>
</template>
