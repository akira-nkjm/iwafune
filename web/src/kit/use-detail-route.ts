import { useNavigate, useSearchParams } from "react-router";

/**
 * 選択と表示方法を URL に載せ、追加ダイアログの指定だけ取り除く。表示方法は `?view=`（`views` のどれか。無ければ
 * 先頭）。`tree` は詳細を横の欄に出す表示（表以外）か。
 */
export function useDetailRoute(base: string, views: readonly string[] = ["table", "tree"]) {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const asked = params.get("view");
  const view = asked && views.includes(asked) ? asked : (views[0] ?? "table");
  const tree = view !== "table";
  const select = (id: string | null) => {
    const next = new URLSearchParams(params);
    next.delete("new");
    next.set("view", view);
    navigate({
      pathname: id ? `${base}/${encodeURIComponent(id)}` : base,
      search: next.toString(),
    });
  };
  const setAdding = (open: boolean) =>
    setParams((previous) => {
      const next = new URLSearchParams(previous);
      if (open) next.set("new", "1");
      else next.delete("new");
      return next;
    });
  return { view, tree, select, adding: params.get("new") === "1", setAdding };
}

/** 行自体を開き元としてフォーカスし、ポインターとキーボードで詳細を開く。 */
export function detailRowProps(id: string, onSelect: (id: string) => void) {
  const open = (event: React.SyntheticEvent<HTMLTableRowElement>) => {
    event.currentTarget.focus();
    onSelect(id);
  };
  return {
    tabIndex: 0,
    onClick: open,
    onKeyDown: (event: React.KeyboardEvent<HTMLTableRowElement>) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      open(event);
    },
  };
}
