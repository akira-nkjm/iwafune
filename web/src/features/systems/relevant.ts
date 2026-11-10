import type { DesignView, PartRow, System } from "@/api/types";

/** この系に関係する部品だけを、この系の属性だけの列で並べる（セルで直接編集） */
export function relevantParts(
  design: DesignView,
  system: System,
): { parts: PartRow[]; attrs: string[] } {
  const attrsInSection = design.defs.attributes
    .filter((a) => a.section === system)
    .map((a) => a.id);
  const want = new Set(attrsInSection);
  const parts = design.parts.filter(
    (p) =>
      (p.role !== null && design.defs.labels.role_system[p.role] === system) ||
      Object.entries(p.values).some(([id, v]) => want.has(id) && v.origin !== "unset"),
  );
  const attrs = attrsInSection.filter((a) => parts.some((p) => p.values[a] !== undefined));
  return { parts, attrs };
}
