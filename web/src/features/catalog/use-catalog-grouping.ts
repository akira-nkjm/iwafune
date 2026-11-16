import { useSearchParams } from "react-router";
import { CATALOG_GROUPINGS, type CatalogGrouping } from "@/kit/tree/tree-nodes";
import { SEARCH } from "@/lib/nav";

/** ツリーの分け方（`?group=`。不明な値と無いときは `catalog`）。ほかの検索部分（選択・表示方法）はそのまま */
export function useCatalogGrouping(): [CatalogGrouping, (next: CatalogGrouping) => void] {
  const [params, setParams] = useSearchParams();
  const asked = params.get(SEARCH.group);
  const grouping = CATALOG_GROUPINGS.find((g) => g === asked) ?? CATALOG_GROUPINGS[0];
  const set = (next: CatalogGrouping) =>
    setParams((previous) => {
      const search = new URLSearchParams(previous);
      search.set(SEARCH.group, next);
      return search;
    });
  return [grouping, set];
}
