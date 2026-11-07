import { useDryRun } from "@/api/ops";
import type { Op } from "@/api/types";
import { useDebounced } from "@/lib/use-debounced";

/**
 * Op を確定する前の検査。規則（ID の書き方・重複・時間割合の合計など）は持たず、POST /ops の dry_run に任せる。
 * 同じ ops の dry_run は 1 回だけ問い合わせる（OpPreview と同じクエリを共有する）。
 */
export function useOpCheck(ops: Op[] | null) {
  const debounced = useDebounced(ops, 250);
  const dry = useDryRun(debounced);
  const settled = JSON.stringify(debounced) === JSON.stringify(ops);
  const diagnostics = dry.error?.diagnostics.filter((d) => d.severity === "error") ?? [];
  const error = dry.error
    ? diagnostics.length > 0
      ? diagnostics.map((d) => d.message).join(" / ")
      : dry.error.message
    : null;
  return {
    /** dry_run の結果（CLI・変わるファイル） */
    result: dry.data ?? null,
    error,
    pending: !!ops && ops.length > 0 && (!settled || dry.isFetching),
    ok: !!ops && ops.length > 0 && settled && !dry.isFetching && !!dry.data,
  };
}
