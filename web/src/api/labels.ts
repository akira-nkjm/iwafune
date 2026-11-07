import { useQuery } from "@tanstack/react-query";
import { localized } from "@/i18n";
import { q } from "./queries";

/**
 * 組み込みの列挙の表示名（`GET /defs` の `labels.enum`。正本は fune-model の labels.toml で、CLI も同じ表を引く）。
 * `ty` は `origin`・`evidence`・`maturity` など。届くまでと表に無いときは値の綴りを返す。
 */
export function useEnumLabel(): (ty: string, value: string) => string {
  const { data } = useQuery(q.defs());
  return (ty, value) => {
    const l = data?.labels.enum[ty]?.[value];
    return (l && localized(l.label, l.label_en)) || value;
  };
}
