import type { DesignView, ViewNodePos } from "@/api/types";

/** 図の位置のスナップ（px） */
export const SNAP = 10;
export const snap = (v: number) => Math.round(v / SNAP) * SNAP;

/** 保存された図の位置（GET /design の views。view/<view>.toml）。x, y は親に対する相対 */
export function savedLayout(design: DesignView, view: string): Map<string, ViewNodePos> {
  const out = new Map<string, ViewNodePos>();
  // 古いサーバーは views を返さないことがある
  const v = (design.views ?? []).find((x) => x.id === view);
  if (!v) return out;
  for (const [id, n] of Object.entries(v.nodes))
    out.set(id, { id, x: n.x, y: n.y, w: n.w ?? null, h: n.h ?? null });
  return out;
}

/** React Flow のノード ID ↔ 図の保存の ID（部品・グループ・ネットの ID そのもの） */
export function viewId(nodeId: string): string {
  return nodeId.replace(/^(card|group|junction):/, "");
}
