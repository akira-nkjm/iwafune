import type { DesignView, Op, PartRow, TreeNode } from "@/api/types";

/**
 * 部品を、親 `parent`（系・グループ・組立品の ID）の下のグループ `group` へまとめる Op（部品ごとに `move_part`）。
 * `under` は親の 1 つだけで足りる（親から上の連鎖は `parts/` の下の置き場所としてサーバーが決める）。
 * グループが無ければ作り、あれば（同じ親の下なら）そこへ入れる。ID の規則と重複はサーバー（dry_run）が検査する。
 */
export function groupOps(parent: string, group: string, partIds: readonly string[]): Op[] {
  return partIds.map((id) => ({ op: "move_part", id, to: group, under: [parent] }));
}

/** グループを解いて、中の部品を親へ戻す Op（空になったフォルダはサーバーが消す） */
export function ungroupOps(parent: string, partIds: readonly string[]): Op[] {
  return partIds.map((id) => ({ op: "move_part", id, to: parent, under: [] }));
}

/** 「グループにまとめる」の選択肢: 部品と同じ親を持つ部品（親が無い部品はまとめられない） */
export function siblingParts(design: DesignView, part: PartRow): PartRow[] {
  if (!part.parent) return [];
  return design.parts.filter((p) => p.parent === part.parent);
}

/**
 * 入れ子のグループ（親が系・グループ・組立品のもの。系そのものは除く）を解けるか。
 * 子が部品だけのときに限る（入れ子のグループが残るフォルダは、先に中のグループを解く）。
 */
export function ungroupParts(
  group: TreeNode,
  nodes: ReadonlyMap<string, TreeNode>,
): string[] | null {
  if (group.node !== "group" || !group.parent) return null;
  const kids = group.children.map((id) => nodes.get(id));
  if (kids.length === 0 || kids.some((n) => n?.node !== "part")) return null;
  return group.children;
}
