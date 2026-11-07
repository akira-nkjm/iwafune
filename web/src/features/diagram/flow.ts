import type { Edge, Node } from "@xyflow/react";
import type { ViewNodePos } from "@/api/types";
import type { DiagramLayout } from "./layout-geometry";
import type { DiagramModel } from "./model";
import { waypointKey } from "./routes";
import { viewId } from "./saved";

function parentsOf(model: DiagramModel): Map<string, string | null> {
  const m = new Map<string, string | null>();
  for (const c of model.containers) m.set(c.id, c.parent);
  for (const c of model.cards) m.set(c.id, c.parent);
  for (const j of model.junctions) m.set(j.id, j.parent);
  return m;
}

function depth(id: string, parents: Map<string, string | null>): number {
  let n = 0;
  for (let p = parents.get(id) ?? null; p; p = parents.get(p) ?? null) n += 1;
  return n;
}

/** 表示モデル + 自動配置（+ 保存した位置）→ React Flow のノード。親を子より先に並べる */
export function flowNodes(
  model: DiagramModel,
  layout: DiagramLayout,
  saved: ReadonlyMap<string, ViewNodePos>,
  opts: { onResized: (id: string, w: number, h: number) => void },
): Node[] {
  const parents = parentsOf(model);
  // 保存した位置と大きさを優先する。枠（`fit`）は、保存した大きさが自動配置で並べた中身より小さければ中身に合わせて
  // 広げる（自動配置の並べ方が変わったとき、中のカードが枠からはみ出さないように）
  const place = (id: string, fit = false) => {
    const s = saved.get(viewId(id));
    const b = layout.boxes.get(id) ?? { x: 0, y: 0, width: 150, height: 52 };
    const size = (stored: number | null | undefined, auto: number) =>
      stored == null ? auto : fit ? Math.max(stored, auto) : stored;
    return {
      position: s ? { x: s.x, y: s.y } : { x: b.x, y: b.y },
      width: size(s?.w, b.width),
      height: size(s?.h, b.height),
    };
  };
  const byDepth = <T extends { id: string }>(xs: T[]) =>
    [...xs].sort((a, b) => depth(a.id, parents) - depth(b.id, parents));
  const out: Node[] = [];
  for (const c of byDepth(model.containers)) {
    const p = place(c.id, true);
    out.push({
      id: c.id,
      type: "container",
      ...p,
      style: { width: p.width, height: p.height },
      parentId: c.parent ?? undefined,
      data: { container: c, onResized: opts.onResized },
      deletable: false,
      zIndex: 0,
    });
  }
  for (const c of byDepth(model.cards)) {
    const p = place(c.id);
    out.push({
      id: c.id,
      type: "card",
      ...p,
      parentId: c.parent ?? undefined,
      deletable: false,
      zIndex: 2 + depth(c.id, parents),
      data: {
        card: c,
        selected: false,
        connectable: c.members.length === 1,
        ports: c.ports.flatMap((port) => {
          const at = layout.ports.get(port.id);
          return at ? [{ ...port, ...at }] : [];
        }),
      },
    });
  }
  for (const j of model.junctions) {
    out.push({
      id: j.id,
      type: "junction",
      ...place(j.id),
      parentId: j.parent ?? undefined,
      deletable: false,
      data: { net: j.net },
      zIndex: 3,
    });
  }
  return out;
}

/**
 * 辺。自動配置の経路を使う（交差の飛び越しは RoutedEdge が描くときに入れる）。
 * 端のノード（かその親）を手で動かした・保存した位置にした辺は、自動配置の経路が合わないので段差の線に戻す。
 */
export function flowEdges(
  model: DiagramModel,
  layout: DiagramLayout,
  moved: ReadonlySet<string>,
  saved: ReadonlyMap<string, ViewNodePos>,
): Edge[] {
  const parents = parentsOf(model);
  const stale = (id: string): boolean => {
    for (let p: string | null = id; p; p = parents.get(p) ?? null)
      if (moved.has(p) || saved.has(viewId(p))) return true;
    return false;
  };
  return model.edges.map((e) => {
    const pts = layout.routes.get(e.id) ?? null;
    const fresh = pts && !stale(e.source.node) && !stale(e.target.node) ? pts : null;
    return {
      id: e.id,
      type: "routed",
      source: e.source.node,
      sourceHandle: e.source.port ?? "j",
      target: e.target.node,
      targetHandle: e.target.port ?? "j",
      zIndex: 5,
      data: {
        routeKey: waypointKey(e),
        nets: e.nets,
        // ラベルの位置は自動配置が線と重ならないように取ったもの。同じ端子・同じプロトコルの線は 1 本にだけ付ける
        labelAt: layout.labels.get(e.id),
        points: fresh,
        color: e.color,
        dashed: e.dashed,
        label: layout.labels.has(e.id) ? e.label : null,
        sourceLabel: e.sourceLabel,
        targetLabel: e.targetLabel,
        selected: false,
      },
    };
  });
}
