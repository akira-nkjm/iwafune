import { useQuery } from "@tanstack/react-query";
import { q } from "@/api/queries";
import type { DimensionInfo } from "@/api/types";
import { localized } from "@/i18n";

/** 次元の表示名（API の `label` / `label_en` から現在の言語で選ぶ）。名前の対が無ければ次元名のまま */
export function dimensionLabelOf(dims: readonly DimensionInfo[], name: string): string {
  const d = dims.find((x) => x.name === name);
  return d ? localized(d.label, d.label_en) || name : name;
}

/**
 * 次元名 → 表示名。単位の一覧（GET /units。画面で共有する）の次元の対から引く。
 * 一覧を読み込むまで、また一覧に無い名前（表示の種類など）は次元名のまま返す。
 */
export function useDimensionLabel(): (name: string) => string {
  const dims = useQuery(q.units(null)).data?.dimensions ?? [];
  return (name) => dimensionLabelOf(dims, name);
}
