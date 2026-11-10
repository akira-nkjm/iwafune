import { useNavigate, useParams, useSearchParams } from "react-router";
import { itemPath, SEARCH } from "@/lib/nav";

/**
 * 一覧と詳細のページの URL の状態: 選んだもの・表示方法・追加ダイアログの開閉。`base` は一覧のページの URL
 * （`lib/nav.ts` の `paths.components()` など）。
 *
 * - 選んだ ID は URL の末尾の区間（ルートの `:id?`。`routes.ts`）。React Router が decode した値をそのまま使う
 *   （重ねて decode しない。`%` を含む ID が壊れる）。`select` は `itemPath` で encode して URL を作る。
 * - 表示方法は `?view=`（`views` のどれか。無ければ先頭）。`tree` は詳細を横の欄に出す表示（表以外）か。
 *   切り替えは `setView`（選んだ ID はそのまま。`kit/ViewSwitch` はこれを受け取り、URL を自分で読まない）。
 * - 追加ダイアログは `?new=1`。選び直すと消す。検索部分の名前は `SEARCH`。
 */
export function useDetailRoute(base: string, views: readonly string[] = ["table", "tree"]) {
  const navigate = useNavigate();
  const { id } = useParams();
  const [params, setParams] = useSearchParams();
  const asked = params.get(SEARCH.view);
  const view = asked && views.includes(asked) ? asked : (views[0] ?? "table");
  const select = (next: string | null) => {
    const search = new URLSearchParams(params);
    search.delete(SEARCH.adding);
    search.set(SEARCH.view, view);
    navigate({
      pathname: next ? itemPath(base, next) : base,
      search: search.toString(),
    });
  };
  // 検索部分の 1 つだけを書き換える（選んだ ID とほかの検索部分はそのまま）
  const patch = (edit: (search: URLSearchParams) => void) =>
    setParams((previous) => {
      const search = new URLSearchParams(previous);
      edit(search);
      return search;
    });
  const setView = (next: string) => patch((search) => search.set(SEARCH.view, next));
  const setAdding = (open: boolean) =>
    patch((search) => (open ? search.set(SEARCH.adding, "1") : search.delete(SEARCH.adding)));
  return {
    view,
    setView,
    tree: view !== "table",
    selected: id ?? null,
    select,
    adding: params.get(SEARCH.adding) === "1",
    setAdding,
  };
}

/** 一覧と詳細のページの URL の状態（`useDetailRoute` の戻り値）。一覧・詳細の部品にはこれをそのまま渡す */
export type DetailRoute = ReturnType<typeof useDetailRoute>;
