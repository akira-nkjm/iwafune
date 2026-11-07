import { useSearchParams } from "react-router";
import { useT } from "@/i18n";
import { Segmented } from "./Segmented";

/** 表示方法を URL に保存する共通スイッチ。 */
export function ViewSwitch() {
  const t = useT().common;
  const [params, setParams] = useSearchParams();
  return (
    <Segmented
      label={t.view}
      value={params.get("view") === "tree" ? "tree" : "table"}
      options={[
        { value: "table", label: t.tableView },
        { value: "tree", label: t.treeView },
      ]}
      onChange={(view) =>
        setParams((previous) => {
          const next = new URLSearchParams(previous);
          next.set("view", view);
          return next;
        })
      }
    />
  );
}
