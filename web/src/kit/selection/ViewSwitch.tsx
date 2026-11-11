import { useT } from "@/i18n";
import { Segmented } from "./Segmented";

/**
 * 表とツリーの表示の切り替え（一覧と詳細のページの見出し）。URL は読まない: 値と変更の関数は呼び手が
 * `useDetailRoute` の `view`・`setView` から渡す。
 */
export function ViewSwitch({
  value,
  onChange,
}: {
  /** 今の表示方法（`tree` 以外は表） */
  value: string;
  /** 選び直したとき */
  onChange: (view: "table" | "tree") => void;
}) {
  const t = useT().common;
  return (
    <Segmented
      label={t.view}
      value={value === "tree" ? "tree" : "table"}
      options={[
        { value: "table", label: t.tableView },
        { value: "tree", label: t.treeView },
      ]}
      onChange={onChange}
    />
  );
}
