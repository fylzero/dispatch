/**
 * The colors a tab or pane can be given from its right-click menu. Each is a
 * space-separated RGB triple, so CSS can lay it over the (possibly
 * see-through) terminal background as a faint wash: rgb(var(--x) / alpha).
 */
export interface LabelColor {
  id: string;
  label: string;
  rgb: string;
}

export const LABEL_COLORS: LabelColor[] = [
  { id: "red", label: "Red", rgb: "232 93 93" },
  { id: "orange", label: "Orange", rgb: "236 142 72" },
  { id: "yellow", label: "Yellow", rgb: "222 184 72" },
  { id: "green", label: "Green", rgb: "84 182 112" },
  { id: "teal", label: "Teal", rgb: "44 170 156" },
  { id: "blue", label: "Blue", rgb: "84 142 232" },
  { id: "purple", label: "Purple", rgb: "152 116 226" },
  { id: "pink", label: "Pink", rgb: "222 104 166" },
];

export function colorRgb(id: string | undefined) {
  return id ? LABEL_COLORS.find((color) => color.id === id)?.rgb : undefined;
}
