import { useDryRun } from "@/api/ops";
import type { Op } from "@/api/types";
import { useDebounced } from "@/lib/use-debounced";

/**
 * Op を確定する前の検査。規則（ID の書き方・重複・時間割合の合計など）は持たず、POST /ops の dry_run に任せる。
 * 同じ ops の dry_run は 1 回だけ問い合わせる（OpPreview と同じクエリを共有する）。
 *
 * **dry_run の弾かれた理由の文（`previewError`）を出す場所は `OpPreview` だけ**（ダイアログの欄の下には出さない。同じ文が
 * 2 か所に出ていた）。欄の下に出すのは、サーバーに問い合わせる前に分かる入力の誤り（呼び手の `made.error`）だけ。
 * 欄の外枠を赤くしたいときは `failed` を使う。この規則は `ops/single-error.test.ts` が見張る。
 */
export function useOpCheck(ops: Op[] | null) {
  const debounced = useDebounced(ops, 250);
  const dry = useDryRun(debounced);
  const settled = JSON.stringify(debounced) === JSON.stringify(ops);
  const diagnostics = dry.error?.diagnostics.filter((d) => d.severity === "error") ?? [];
  const previewError = dry.error
    ? diagnostics.length > 0
      ? diagnostics.map((d) => d.message).join(" / ")
      : dry.error.message
    : null;
  return {
    /** dry_run の結果（CLI・変わるファイル） */
    result: dry.data ?? null,
    /** dry_run が弾かれたか（理由の文は出さない。`previewError` は `OpPreview` だけが使う） */
    failed: !!dry.error,
    /** 弾かれた理由の文（`OpPreview` だけが表示する） */
    previewError,
    pending: !!ops && ops.length > 0 && (!settled || dry.isFetching),
    ok: !!ops && ops.length > 0 && settled && !dry.isFetching && !!dry.data,
  };
}
