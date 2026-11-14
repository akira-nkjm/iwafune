import { useMutation, useQueryClient } from "@tanstack/react-query";
import { type ApiError, api } from "./client";
import { invalidatedByOps } from "./keys";
import type { CatalogFetchRequest, CatalogSyncResult } from "./types";

/**
 * 共有カタログを取得する（`POST /catalog/fetch`。CLI の `fune catalog fetch` と同じ service の関数）。
 * lock とキャッシュが変わると設計（品・出典・診断）が変わるので、編集の後と同じ範囲を読み直す。
 */
export function useFetchCatalogs() {
  const qc = useQueryClient();
  return useMutation<CatalogSyncResult, ApiError, CatalogFetchRequest>({
    mutationFn: (req) => api<CatalogSyncResult>("/catalog/fetch", { method: "POST", body: req }),
    onSuccess: async () => {
      await Promise.all(invalidatedByOps.map((k) => qc.invalidateQueries({ queryKey: k })));
    },
  });
}
