import { useEnumLabel } from "@/api/labels";
import type { CatalogItemView } from "@/api/types";
import { useT } from "@/i18n";
import { DetailSection } from "@/kit/detail/DetailSection";
import { PropertyList } from "@/kit/detail/PropertyList";
import { PropertyRow } from "@/kit/detail/PropertyRow";
import { Pill } from "@/kit/status/Pill";

/** 種別から決まる読み取り専用の区分: ポートの雛形（ポート名 → ネットの種類）と状態の語彙。空の区分は出さない */
export function CatalogInherited({ item }: { item: CatalogItemView }) {
  const t = useT().catalog;
  const label = useEnumLabel();
  const ports = Object.entries(item.ports);
  return (
    <>
      {ports.length > 0 && (
        <DetailSection title={t.ports}>
          <PropertyList>
            {ports.map(([name, kind]) => (
              <PropertyRow key={name} label={<span className="font-mono text-ink">{name}</span>}>
                <span className="text-ink-2">{label("net_kind", kind)}</span>
              </PropertyRow>
            ))}
          </PropertyList>
        </DetailSection>
      )}
      {item.states.length > 0 && (
        <DetailSection title={t.states}>
          <ul className="flex flex-wrap gap-1">
            {item.states.map((s) => (
              <li key={s}>
                <Pill dot={false}>{s}</Pill>
              </li>
            ))}
          </ul>
        </DetailSection>
      )}
    </>
  );
}
