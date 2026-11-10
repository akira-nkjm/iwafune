import { useEnumOptions } from "@/api/labels";
import { useApplyOps, useSetText, useUnsetText } from "@/api/ops";
import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { CommitInput } from "@/kit/CommitInput";
import { NativeSelect } from "@/kit/NativeSelect";
import { PropertyList } from "@/kit/PropertyList";
import { PropertyRow } from "@/kit/PropertyRow";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { fieldPath, idWithName } from "@/lib/design";
import { useFieldError } from "@/lib/field-errors";

/** 検証で弾かれた理由（その欄のパスに付いたもの）を欄の下に出す */
function FieldError({ path }: { path: string | null }) {
  const error = useFieldError(path);
  return error ? <p className="mt-0.5 text-[11px] text-neg-ink">{error}</p> : null;
}

/**
 * 基本: 種別・カタログ（トレード候補の切り替え）・トレード候補の一覧・員数・成熟度・所属（グループ）。
 * フィールドの欄は、打った（選んだ）文字列と API のパス（`PartRow.field_paths`）をそのまま `fune set`・`fune unset` と
 * 同じ service の関数に渡す（空なら消す）。員数・成熟度・一覧の読み方と検査は service で、Web は `set_field` の Op を組み立てない
 */
export function BasicSection({ design, part }: { design: DesignView; part: PartRow }) {
  const apply = useApplyOps();
  const setText = useSetText();
  const unsetText = useUnsetText();
  const t = useT().components.basic;
  const maturity = useEnumOptions("maturity");
  const path = (field: string) => fieldPath(part, field);
  const save = (field: string, value: string, summary: string) => {
    const p = path(field);
    if (!p) return;
    if (value) setText.mutate({ path: p, value, summary });
    else unsetText.mutate({ path: p, summary });
  };
  const candidates =
    part.candidates.length > 0 ? part.candidates : part.catalog ? [part.catalog] : [];
  // 一覧の書き方は `fune set <部品>.candidates "a, b"` と同じカンマ区切り（読むのは service の edit::list_from_text）
  const candidateText = part.candidates.join(", ");
  return (
    <PropertyList>
      <PropertyRow label={t.kind}>
        <span className="font-mono">
          {part.kind}
          {part.role && part.role !== part.kind && (
            <span className="text-ink-3"> （{part.role}）</span>
          )}
        </span>
      </PropertyRow>
      <PropertyRow label={t.catalog}>
        {candidates.length > 1 ? (
          <NativeSelect
            size="sm"
            aria-label={t.candidate}
            value={part.catalog ?? ""}
            onChange={(e) =>
              save("catalog", e.target.value, t.candidateSummary(part.id, e.target.value))
            }
            options={candidates.map((c) => ({ value: c, label: c }))}
          />
        ) : (
          <span className="font-mono">{part.catalog ?? t.kindOnly}</span>
        )}
        <FieldError path={path("catalog")} />
      </PropertyRow>
      <PropertyRow label={t.candidates} title={t.candidatesHint}>
        <CommitInput
          label={t.candidates}
          title={t.candidatesHint}
          mono
          value={candidateText}
          onCommit={(v) => save("candidates", v, t.candidatesSummary(part.id))}
        />
        <FieldError path={path("candidates")} />
      </PropertyRow>
      <PropertyRow label={t.count}>
        {/* 空にしただけなら元に戻す（員数を 1 に戻すときは 1 と打つ） */}
        <CommitInput
          type="number"
          min={1}
          step={1}
          label={t.count}
          allowEmpty={false}
          value={String(part.count)}
          onCommit={(v) => save("count", v, t.countSummary(part.id, v))}
          className="num w-20 text-right"
        />
        <FieldError path={path("count")} />
      </PropertyRow>
      <PropertyRow label={t.maturity}>
        <NativeSelect
          size="sm"
          aria-label={t.maturity}
          value={part.maturity ?? ""}
          placeholder={t.maturityUnset}
          onChange={(e) =>
            save("maturity", e.target.value, t.maturitySummary(part.id, e.target.value))
          }
          options={maturity}
        />
        <FieldError path={path("maturity")} />
      </PropertyRow>
      <PropertyRow label={t.parent}>
        <span className="flex items-center gap-2">
          <SubsystemDot group={part.subsystem} role={part.role} />
          <NativeSelect
            size="sm"
            aria-label={t.parentGroup}
            value={part.parent ?? ""}
            onChange={(e) =>
              apply.mutate({
                ops: [{ op: "move_part", id: part.id, to: e.target.value || null }],
                summary: t.moveSummary(part.id, e.target.value),
              })
            }
            options={design.tree
              .filter((n) => n.id !== part.id)
              .filter((n) => n.node === "group" || n.children.length > 0 || n.id === part.parent)
              .map((n) => ({ value: n.id, label: idWithName(n) }))}
          />
        </span>
      </PropertyRow>
      <PropertyRow label={t.file}>
        <span className="truncate font-mono text-[11.5px] text-ink-3">{part.file}</span>
      </PropertyRow>
    </PropertyList>
  );
}
