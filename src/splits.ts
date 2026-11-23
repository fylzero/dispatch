/**
 * The split-pane tree inside a terminal tab, as plain data and pure helpers.
 *
 * A tab's layout is a tree: a leaf is one terminal, a split lays its children
 * out side by side (axis "x", ⌘D) or stacked (axis "y", ⌘⇧D). Splitting a pane
 * only ever divides that pane, like iTerm and tmux. `weight` is a node's share
 * of its parent split; only ratios between siblings matter.
 */

export type Axis = "x" | "y";

export interface PaneNode {
  kind: "pane";
  id: string;
  weight: number;
}

export interface SplitNode {
  kind: "split";
  id: string;
  axis: Axis;
  weight: number;
  children: LayoutNode[];
}

export type LayoutNode = PaneNode | SplitNode;

/** A box as fractions (0–1) of the tab's area. */
export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface PlacedPane {
  id: string;
  rect: Rect;
}

/** The line between child `index` and `index + 1` of a split. */
export interface PlacedDivider {
  key: string;
  split: SplitNode;
  index: number;
  axis: Axis;
  /** The split's own box, so a drag can turn pixels into weight. */
  area: Rect;
  /** Where the line sits: an x for side-by-side splits, a y for stacked ones. */
  at: number;
  /** The line's extent along the other axis. */
  from: number;
  span: number;
}

export type Direction = "left" | "right" | "up" | "down";

const total = (nodes: LayoutNode[]) => nodes.reduce((sum, node) => sum + node.weight, 0);

/** Every pane's box and every divider, for drawing the tab. */
export function layout(root: LayoutNode) {
  const panes: PlacedPane[] = [];
  const dividers: PlacedDivider[] = [];
  const walk = (node: LayoutNode, rect: Rect) => {
    if (node.kind === "pane") {
      panes.push({ id: node.id, rect });
      return;
    }
    const sum = total(node.children) || 1;
    let offset = 0;
    node.children.forEach((child, index) => {
      const share = child.weight / sum;
      const box =
        node.axis === "x"
          ? { x: rect.x + rect.w * offset, y: rect.y, w: rect.w * share, h: rect.h }
          : { x: rect.x, y: rect.y + rect.h * offset, w: rect.w, h: rect.h * share };
      if (index > 0) {
        dividers.push({
          key: `${node.id}:${index - 1}`,
          split: node,
          index: index - 1,
          axis: node.axis,
          area: rect,
          at: node.axis === "x" ? box.x : box.y,
          from: node.axis === "x" ? rect.y : rect.x,
          span: node.axis === "x" ? rect.h : rect.w,
        });
      }
      walk(child, box);
      offset += share;
    });
  };
  walk(root, { x: 0, y: 0, w: 1, h: 1 });
  return { panes, dividers };
}

/** The split holding `id` directly, and where in it. Null for the root. */
function parentOf(root: LayoutNode, id: string): { parent: SplitNode; index: number } | null {
  if (root.kind === "pane") return null;
  const index = root.children.findIndex((child) => child.id === id);
  if (index !== -1) return { parent: root, index };
  for (const child of root.children) {
    const found = parentOf(child, id);
    if (found) return found;
  }
  return null;
}

export function countPanes(node: LayoutNode): number {
  return node.kind === "pane" ? 1 : node.children.reduce((sum, child) => sum + countPanes(child), 0);
}

export function firstPane(node: LayoutNode): string {
  return node.kind === "pane" ? node.id : firstPane(node.children[0]);
}

/**
 * Divides the pane `target` along `axis`, putting `added` after it. When the
 * pane already sits in a split along that axis, the new pane joins that split
 * and its panes share the space evenly; otherwise the pane becomes a new
 * two-way split. Returns the new root.
 */
export function splitPane(root: LayoutNode, target: string, axis: Axis, added: PaneNode, newSplitId: string) {
  const found = parentOf(root, target);
  if (found && found.parent.axis === axis) {
    found.parent.children.splice(found.index + 1, 0, added);
    found.parent.children.forEach((child) => (child.weight = 1));
    return root;
  }
  const pane = found ? found.parent.children[found.index] : root;
  const split: SplitNode = {
    kind: "split",
    id: newSplitId,
    axis,
    weight: pane.weight,
    children: [{ ...pane, weight: 1 } as LayoutNode, added],
  };
  if (!found) return split;
  found.parent.children[found.index] = split;
  return root;
}

/** How many panes the split along `axis` would hold after splitting `target`. */
export function siblingsAfterSplit(root: LayoutNode, target: string, axis: Axis) {
  const found = parentOf(root, target);
  return found && found.parent.axis === axis ? found.parent.children.length + 1 : 2;
}

/**
 * Removes pane `id`. A split left with one child is replaced by that child,
 * and a child split along the same axis as its new parent is merged into it,
 * so the tree stays as flat as the screen looks. Returns the new root (null
 * when the last pane went) and the pane that should take focus.
 */
export function removePane(root: LayoutNode, id: string): { root: LayoutNode | null; focus: string | null } {
  const found = parentOf(root, id);
  if (!found) return { root: null, focus: null };
  const { parent, index } = found;
  parent.children.splice(index, 1);
  const neighbor = parent.children[Math.min(index, parent.children.length - 1)];
  const focus = firstPane(neighbor);
  if (parent.children.length > 1) return { root, focus };

  /* One child left: it takes the split's place. */
  const only = parent.children[0];
  only.weight = parent.weight;
  const up = parentOf(root, parent.id);
  if (!up) return { root: only, focus };
  if (only.kind === "split" && only.axis === up.parent.axis) {
    const sum = total(only.children) || 1;
    const merged = only.children.map((child) => ({ ...child, weight: (child.weight / sum) * only.weight }));
    up.parent.children.splice(up.index, 1, ...merged);
  } else {
    up.parent.children[up.index] = only;
  }
  return { root, focus };
}

/**
 * The pane next to `from` in a direction: the nearest one across that edge
 * that overlaps it, preferring the one level with its middle.
 */
export function neighborOf(root: LayoutNode, from: string, direction: Direction): string | null {
  const { panes } = layout(root);
  const current = panes.find((pane) => pane.id === from);
  if (!current) return null;
  const c = current.rect;
  const eps = 1e-6;
  const horizontal = direction === "left" || direction === "right";
  const middle = horizontal ? c.y + c.h / 2 : c.x + c.w / 2;
  let best: { id: string; distance: number; offset: number } | null = null;
  for (const pane of panes) {
    if (pane.id === from) continue;
    const r = pane.rect;
    let distance: number;
    if (direction === "left") distance = c.x - (r.x + r.w);
    else if (direction === "right") distance = r.x - (c.x + c.w);
    else if (direction === "up") distance = c.y - (r.y + r.h);
    else distance = r.y - (c.y + c.h);
    if (distance < -eps) continue;
    const overlaps = horizontal ? r.y < c.y + c.h - eps && r.y + r.h > c.y + eps : r.x < c.x + c.w - eps && r.x + r.w > c.x + eps;
    if (!overlaps) continue;
    const start = horizontal ? r.y : r.x;
    const end = horizontal ? r.y + r.h : r.x + r.w;
    const offset = middle < start ? start - middle : middle > end ? middle - end : 0;
    if (!best || distance < best.distance - eps || (Math.abs(distance - best.distance) <= eps && offset < best.offset)) {
      best = { id: pane.id, distance, offset };
    }
  }
  return best?.id ?? null;
}
