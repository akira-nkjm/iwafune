import type { DesignView, System } from "@/api/types";
import type { DiagramModel } from "./model";

/** カードとそれに接する線を隠す。隠れたハブを別の端に置き換えず、設計上の接続を維持する。 */
export function visibleModel(
  model: DiagramModel,
  design: DesignView,
  hidden: ReadonlySet<System>,
): DiagramModel {
  const parts = new Map(design.parts.map((p) => [p.id, p]));
  const cards = model.cards.filter((c) =>
    c.members.some((id) => {
      const system = parts.get(id)?.system;
      return !system || !hidden.has(system);
    }),
  );
  const visible = new Set(cards.map((c) => c.id));
  const parents = new Map([...model.containers, ...model.cards].map((n) => [n.id, n.parent]));
  const removed = new Set(model.cards.filter((c) => !visible.has(c.id)).map((c) => c.id));
  const parentOf = (parent: string | null): string | null => {
    const seen = new Set<string>();
    while (parent && removed.has(parent) && !seen.has(parent)) {
      seen.add(parent);
      parent = parents.get(parent) ?? null;
    }
    return parent && !seen.has(parent) ? parent : null;
  };
  const junctionIds = new Set(model.junctions.map((j) => j.id));
  const edges = model.edges.filter((e) =>
    [e.source.node, e.target.node].every((id) => visible.has(id) || junctionIds.has(id)),
  );
  // 線が 1 本しか残らない分岐点も残す（可視の端に別の端があるような線を作らない）。
  const junctions = model.junctions
    .filter((j) => edges.some((e) => e.source.node === j.id || e.target.node === j.id))
    .map((j) => ({ ...j, parent: parentOf(j.parent) }));
  const used = new Set<string>();
  for (const node of [...cards, ...junctions]) {
    let parent = parentOf(node.parent);
    while (parent && !used.has(parent)) {
      used.add(parent);
      parent = parentOf(parents.get(parent) ?? null);
    }
  }
  return {
    ...model,
    cards: cards.map((c) => ({ ...c, parent: parentOf(c.parent) })),
    containers: model.containers
      .filter((c) => used.has(c.id))
      .map((c) => ({ ...c, parent: parentOf(c.parent) })),
    junctions,
    edges,
  };
}
