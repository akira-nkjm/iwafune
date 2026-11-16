import { useMutation, useQueryClient } from "@tanstack/react-query";
import { type ApiError, api } from "./client";
import { invalidatedByOps } from "./keys";
import type { CatalogFetchRequest, CatalogSyncResult, CatalogUpdateRequest } from "./types";

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

/** `POST /catalog/update` を呼ぶ（テストから直接呼べるよう、フックから分けてある） */
export function updateCatalog(req: CatalogUpdateRequest): Promise<CatalogSyncResult> {
  return api<CatalogSyncResult>("/catalog/update", { method: "POST", body: req });
}

/**
 * 共有カタログ 1 つの lock を `rev` の最新の commit に上げる（`POST /catalog/update`。CLI の `fune catalog update` と同じ service の関数）。
 * 結果に変わった品と値の差分が入る。取得と同じ範囲を読み直す。
 */
export function useUpdateCatalog() {
  const qc = useQueryClient();
  return useMutation<CatalogSyncResult, ApiError, CatalogUpdateRequest>({
    mutationFn: updateCatalog,
    onSuccess: async () => {
      await Promise.all(invalidatedByOps.map((k) => qc.invalidateQueries({ queryKey: k })));
    },
  });
}
