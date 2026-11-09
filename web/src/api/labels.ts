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

/**
 * 列挙の値の選択肢（`labels.enum` の値を `labels.enum_order` の順に。順が無い型は綴りの順）。表示は「表示名（値）」。
 * 成熟度（`maturity`）などの選択肢は、値の一覧を手で書かずにこれで作る。届くまでは空。
 */
export function useEnumOptions(ty: string): { value: string; label: string }[] {
  const { data } = useQuery(q.defs());
  const labels = data?.labels;
  const values = labels?.enum_order[ty] ?? Object.keys(labels?.enum[ty] ?? {}).sort();
  return values.map((value) => {
    const l = labels?.enum[ty]?.[value];
    const name = (l && localized(l.label, l.label_en)) || value;
    return { value, label: name !== value ? `${name}（${value}）` : value };
  });
}
