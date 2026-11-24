import { analysisOutput } from "@/api/analysis-output";
import type { AnalysisRunResult, FindingKind, FindingState } from "@/api/types";

/** 幾何の所見 1 件（箱の重なり 1 組、または円錐に入る部品 1 つ）。API の出力をそのまま並べ替えずに写したもの（判定はしない） */
export interface GeometryFinding {
  kind: FindingKind;
  /** 重なっている 2 つの部品の 1 つめ。円錐では担い手 */
  first: string;
  /** 重なっている 2 つの部品の 2 つめ。円錐では円錐に入る部品 */
  second: string;
  /** 承知済みか（API の `acknowledged`。承知済みでも所見は残る） */
  acknowledged: boolean;
  /** 承知した理由（承知済みのとき） */
  reason: string | null;
  /** 承知が形態を指していれば、その形態（取り消すときに同じ承知を指す。無ければすべての配置に効く承知） */
  acknowledgedConfiguration: string | null;
}

/** 所見を出す 2 つの解析（所見の取り出しはこの表だけ） */
export const FINDING_ANALYSES = ["structure.interference", "structure.cone_clearance"] as const;

/** 解析の結果から幾何の所見を取り出す（ほかの解析・入力不足・失敗は空）。並びは API の出力の順 */
export function geometryFindings(result: AnalysisRunResult | null | undefined): GeometryFinding[] {
  const overlaps = analysisOutput(result, "structure.interference")?.overlaps;
  if (overlaps)
    return overlaps.map((p) => ({
      kind: "interference",
      first: p.first,
      second: p.second,
      acknowledged: p.acknowledged,
      reason: p.reason ?? null,
      acknowledgedConfiguration: p.acknowledged_configuration ?? null,
    }));
  const cones = analysisOutput(result, "structure.cone_clearance")?.cones;
  return (cones ?? []).flatMap((c) =>
    c.obstructions.map((o) => ({
      kind: "cone_clearance" as const,
      first: c.id,
      second: o.part,
      acknowledged: o.acknowledged,
      reason: o.reason ?? null,
      acknowledgedConfiguration: o.acknowledged_configuration ?? null,
    })),
  );
}

/** 所見の組の識別（一覧の行のキー。種類と 2 つの部品。形態は結果が 1 つの配置のものなので要らない） */
export function findingKey(f: Pick<GeometryFinding, "kind" | "first" | "second">): string {
  return JSON.stringify([f.kind, f.first, f.second]);
}

/**
 * 結果の `part_findings`（API が決めた、部品ごとの状態）を部品の ID で引ける表にする。2 つの解析に同じ部品が載るときは、
 * 承知でない所見が残るほう（`open`）を残す（状態は API の値のまま。ここでは所見を数えない）。
 */
export function findingStates(results: readonly AnalysisRunResult[]): Map<string, FindingState> {
  const states = new Map<string, FindingState>();
  for (const result of results)
    for (const { part, state } of result.part_findings ?? [])
      if (states.get(part) !== "open") states.set(part, state);
  return states;
}
