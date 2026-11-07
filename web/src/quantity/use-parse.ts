import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { keys } from "@/api/keys";
import type { ParsedQuantity, ParseQuantityRequest } from "@/api/types";
import { useDebounced } from "@/lib/use-debounced";

/**
 * 入力文字列を POST /units/parse で解析する（単位の解析と次元の検査は sat-units だけが行う）。
 * 結果は SI での表示（`= 0.018 N*m*s`）か、次元違いなどのエラー文。
 */
export function useParseQuantity(input: string, dimension: string | null) {
  const text = useDebounced(input.trim(), 180);
  const query = useQuery({
    queryKey: keys.parse(text, dimension),
    queryFn: () => {
      const body: ParseQuantityRequest = { input: text, dimension };
      return api<ParsedQuantity>("/units/parse", { method: "POST", body });
    },
    enabled: text.length > 0,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  });
  const settled = text === input.trim();
  return {
    parsed: query.data ?? null,
    error: query.error?.message ?? null,
    pending: text.length > 0 && (!settled || query.isFetching),
    empty: input.trim().length === 0,
  };
}
