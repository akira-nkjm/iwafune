import { useEnumOptions } from "@/api/labels";
import { useSetText, useUnsetText } from "@/api/ops";
import type { CatalogItemView } from "@/api/types";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/detail/PropertyList";
import { PropertyRow } from "@/kit/detail/PropertyRow";
import { CommitInput } from "@/kit/forms/CommitInput";
import { NativeSelect } from "@/kit/forms/NativeSelect";

/** 文字列で書くフィールドの名前（`fune catalog set <品> <フィールド> <値>` のフィールド） */
type TextField = "label" | "label_en" | "manufacturer" | "part_number" | "datasheet" | "source";

/**
 * カタログ品の基本の欄（名前・英語名・メーカー・型番・成熟度・データシート・出典）。欄を離れると、打った文字列のまま
 * `fune catalog set` と同じ service の関数に渡す（空なら `fune catalog unset`）
 */
export function CatalogFields({ item }: { item: CatalogItemView }) {
  const t = useT();
  const maturity = useEnumOptions("maturity");
  const setText = useSetText();
  const unsetText = useUnsetText();
  const readOnly = !item.editable;
  const save = (field: TextField | "maturity", name: string, value: string) => {
    const summary = t.catalog.summaryField(item.id, name);
    if (value) setText.mutate({ item: item.id, path: field, value, summary });
    else unsetText.mutate({ item: item.id, path: field, summary });
  };
  const rows: { key: TextField; label: string; value: string | null }[] = [
    { key: "label", label: t.catalog.label, value: item.label },
    { key: "label_en", label: t.catalog.labelEn, value: item.label_en },
    { key: "manufacturer", label: t.catalog.manufacturer, value: item.manufacturer },
    { key: "part_number", label: t.catalog.partNumber, value: item.part_number },
    { key: "datasheet", label: t.catalog.datasheet, value: item.datasheet },
    { key: "source", label: t.catalog.source, value: item.source },
  ];
  return (
    <PropertyList>
      <PropertyRow label={t.catalog.kind}>
        <span className="font-mono">{item.kind}</span>
      </PropertyRow>
      {rows.map((r) => (
        <PropertyRow key={r.key} label={r.label}>
          <CommitInput
            label={r.label}
            value={r.value ?? ""}
            disabled={readOnly}
            onCommit={(v) => save(r.key, r.label, v)}
          />
        </PropertyRow>
      ))}
      <PropertyRow label={t.catalog.maturity}>
        <NativeSelect
          size="sm"
          aria-label={t.catalog.maturity}
          value={item.maturity ?? ""}
          disabled={readOnly}
          placeholder="—"
          onChange={(e) => save("maturity", t.catalog.maturity, e.target.value)}
          options={maturity}
        />
      </PropertyRow>
    </PropertyList>
  );
}
