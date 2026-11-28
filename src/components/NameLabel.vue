<script setup lang="ts">
import { computed } from "vue";

/**
 * A tab or pane name with anything typed in parentheses shown as a badge:
 * "api (staging)" reads as "api" with a "staging" badge. Only names someone
 * typed get badges (pass badges=false for a shell's own title), and only once
 * saved: the rename field shows the raw text.
 */
const props = withDefaults(defineProps<{ text: string; badges?: boolean }>(), { badges: true });

type Part = { kind: "text" | "badge"; value: string };

const parts = computed<Part[]>(() => {
  if (!props.badges) return [{ kind: "text", value: props.text }];
  const result: Part[] = [];
  let last = 0;
  for (const match of props.text.matchAll(/\(([^()]*\S[^()]*)\)/g)) {
    const before = props.text.slice(last, match.index).trim();
    if (before) result.push({ kind: "text", value: before });
    result.push({ kind: "badge", value: match[1].trim() });
    last = match.index + match[0].length;
  }
  const rest = props.text.slice(last).trim();
  if (rest) result.push({ kind: "text", value: rest });
  return result.length ? result : [{ kind: "text", value: props.text }];
});
</script>

<template>
  <template v-for="(part, index) in parts" :key="index">
    <span v-if="part.kind === 'badge'" class="name-badge">{{ part.value }}</span>
    <template v-else>{{ part.value }}</template>
    <template v-if="index < parts.length - 1">{{ " " }}</template>
  </template>
</template>
