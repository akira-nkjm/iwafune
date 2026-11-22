import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { formatKey, formatQuantity } from "./format-batch";

/**
 * SI の値の表示文字列（例: 0.0182 W → "18.2 mW"）。単位と桁は Rust（sat_units::format）が選ぶ。
 * unit を渡すとその単位で（比を "%" で出すなど）。届くまでと失敗したときは null。
 * 値が変わったとき（再生で刻々と変わる温度など）は、新しい文字列が届くまで前の文字列を出し続ける（待ちの印に戻さない。表示が点滅しないように）。
 */
export function useFormatted(
  si: number | null | undefined,
  dimension: string | null | undefined,
  unit: string | null = null,
): string | null {
  const enabled = typeof si === "number" && Number.isFinite(si) && !!dimension;
  const item = { si: enabled ? (si as number) : 0, dimension: dimension ?? "", unit };
  const q = useQuery({
    queryKey: formatKey(item),
    queryFn: () => formatQuantity(item),
    enabled,
    staleTime: Number.POSITIVE_INFINITY,
    gcTime: 10 * 60_000,
    retry: false,
    placeholderData: keepPreviousData,
  });
  // 値が無いときの問い合わせのキーは「0」と同じなので、ほかの値 0 の結果を使い回さないようにする
  return enabled ? (q.data ?? null) : null;
}
