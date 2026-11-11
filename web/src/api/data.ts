import { useMutation, useQueryClient } from "@tanstack/react-query";
import { type ApiError, api, seg } from "./client";
import { keys } from "./keys";
import type { DataStatus } from "./types";

/**
 * 外部データ（DE440）を取得して読み込む（`POST /data/{id}/fetch`。CLI の `fune data fetch` と同じ service の関数）。
 * 返ってきた状態をそのまま状態の問い合わせに入れ、天体の暦が変わるので解析の結果を読み直す。
 */
export function useFetchData() {
  const qc = useQueryClient();
  return useMutation<DataStatus, ApiError, string>({
    mutationFn: (id) => api<DataStatus>(`/data/${seg(id)}/fetch`, { method: "POST" }),
    onSuccess: (status) => {
      qc.setQueryData(keys.dataStatus, status);
      void qc.invalidateQueries({ queryKey: keys.analysisResults });
    },
  });
}
