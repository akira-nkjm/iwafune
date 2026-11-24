import type { QueryClient } from "@tanstack/react-query";
import { invalidatedByOps, type Retired, refersToRetired } from "./keys";

/**
 * 設計から導出する問い合わせを読み直す（編集・取り消し・共有カタログの取得のあとの共通の入口）。
 * `retired`（消した・名前を変えた対象。`retiredBy`）をキーに持つ問い合わせは読み直さず、キャッシュから外す。
 * 読み直すと、画面がまだ古い対象を見ている間（移る操作は描画のあとに確定する）に、無い対象を読みに行って 404 になる。
 * まだ見ている問い合わせ（購読者がいるもの）は外せないので、そのまま残す（購読者がいなくなれば不要な問い合わせとして捨てられる）。
 */
export async function refreshDerived(qc: QueryClient, retired?: Retired): Promise<void> {
  const keep = (key: readonly unknown[]) => !retired || !refersToRetired(key, retired);
  await Promise.all(
    invalidatedByOps.map((k) =>
      qc.invalidateQueries({ queryKey: k, predicate: (query) => keep(query.queryKey) }),
    ),
  );
  if (retired)
    qc.removeQueries({
      predicate: (query) => query.getObserversCount() === 0 && !keep(query.queryKey),
    });
}
