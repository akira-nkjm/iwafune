import { useEnumOptions } from "@/api/labels";
import { useApplyOps } from "@/api/ops";
import type { DesignView, Maturity, PartField, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/NativeSelect";
import { PropertyList } from "@/kit/PropertyList";
import { PropertyRow } from "@/kit/PropertyRow";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { idWithName } from "@/lib/design";

/** 基本: 種別・カタログ（トレード候補の切り替え）・員数・成熟度・所属（グループ） */
export function BasicSection({ design, part }: { design: DesignView; part: PartRow }) {
  const apply = useApplyOps();
  const t = useT().components.basic;
  const maturity = useEnumOptions("maturity");
  const field = (f: PartField, summary: string) =>
    apply.mutate({ ops: [{ op: "set_field", part: part.id, field: f }], summary });
  const candidates =
    part.candidates.length > 0 ? part.candidates : part.catalog ? [part.catalog] : [];
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
              field(
                { field: "catalog", value: e.target.value },
                t.candidateSummary(part.id, e.target.value),
              )
            }
            options={candidates.map((c) => ({ value: c, label: c }))}
          />
        ) : (
          <span className="font-mono">{part.catalog ?? t.kindOnly}</span>
        )}
      </PropertyRow>
      <PropertyRow label={t.count}>
        <input
          type="number"
          min={1}
          step={1}
          aria-label={t.count}
          defaultValue={part.count}
          key={part.count}
          onBlur={(e) => {
            const n = Number.parseInt(e.target.value, 10);
            if (Number.isInteger(n) && n >= 1 && n !== part.count)
              field({ field: "count", value: n }, t.countSummary(part.id, n));
          }}
          className="num h-7 w-20 rounded-[8px] border border-line-strong px-2 text-right"
        />
      </PropertyRow>
      <PropertyRow label={t.maturity}>
        <NativeSelect
          size="sm"
          aria-label={t.maturity}
          value={part.maturity ?? ""}
          placeholder={t.maturityUnset}
          onChange={(e) =>
            field(
              { field: "maturity", value: (e.target.value || null) as Maturity | null },
              t.maturitySummary(part.id, e.target.value),
            )
          }
          options={maturity}
        />
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
