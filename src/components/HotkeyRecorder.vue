<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import * as api from "../api";
import { acceleratorFromEvent, acceleratorKeys } from "../hotkeys";

const props = defineProps<{ value: string; label: string }>();
const emit = defineEmits<{ change: [accelerator: string]; invalid: [message: string] }>();

const recording = ref(false);
const preview = ref("");
const button = ref<HTMLButtonElement | null>(null);

function start() {
  if (recording.value) return;
  recording.value = true;
  preview.value = "";
  /* The current combo must let go of its keys, or pressing it would hide the window. */
  void api.pauseToggleHotkey(true).catch(() => undefined);
  window.addEventListener("keydown", onKey, true);
  window.addEventListener("keyup", onKeyUp, true);
}

function stop() {
  if (!recording.value) return;
  recording.value = false;
  preview.value = "";
  void api.pauseToggleHotkey(false).catch(() => undefined);
  window.removeEventListener("keydown", onKey, true);
  window.removeEventListener("keyup", onKeyUp, true);
}

function onKey(event: KeyboardEvent) {
  event.preventDefault();
  event.stopPropagation();
  const plain = !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey;
  if (plain && event.key === "Escape") {
    stop();
    return;
  }
  if (plain && (event.key === "Backspace" || event.key === "Delete")) {
    stop();
    emit("change", "");
    return;
  }
  const result = acceleratorFromEvent(event);
  if (result.kind === "pending") {
    preview.value = result.preview;
  } else if (result.kind === "invalid") {
    emit("invalid", result.message);
  } else {
    stop();
    emit("change", result.accelerator);
    button.value?.blur();
  }
}

function onKeyUp(event: KeyboardEvent) {
  const result = acceleratorFromEvent(event);
  if (result.kind === "pending") preview.value = result.preview;
}

onBeforeUnmount(stop);
</script>

<template>
  <button
    ref="button"
    type="button"
    class="hotkey"
    :class="{ recording, unset: !value && !recording }"
    :aria-label="`${props.label} shortcut`"
    :title="recording ? 'Press the new shortcut · Esc cancels · Delete clears' : 'Click to record a new shortcut'"
    @click="start"
    @blur="stop"
  >
    <template v-if="recording">
      <span>{{ preview || "Press keys…" }}</span>
    </template>
    <template v-else-if="value">
      <kbd v-for="(key, i) in acceleratorKeys(value)" :key="i">{{ key }}</kbd>
    </template>
    <template v-else>Not set</template>
  </button>
</template>
