import { useQuery } from "@tanstack/react-query";
import { Pencil, Plus, Trash2, X } from "lucide-react";
import { useState } from "react";
import { q } from "@/api/queries";
import type { DesignView, PartRow, StateAssign } from "@/api/types";
import { localized, useT } from "@/i18n";
import { SeverityPill } from "@/kit/SeverityPill";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { nodeLabel } from "@/lib/design";
import { subsystemTone } from "@/lib/subsystems";
import { toneBorder, toneHex } from "@/lib/tone";
import { Button } from "@/ui/button";
import { AddParameterDialog } from "./AddParameterDialog";
import { AttrTable } from "./AttrTable";
import { BasicSection } from "./BasicSection";
import { DeletePartDialog } from "./DeletePartDialog";
import { DetailSection } from "./DetailSection";
import { PartModes } from "./PartModes";
import { PlacementSection } from "./PlacementSection";
import { PortsSection } from "./PortsSection";
import { RenamePartDialog } from "./RenamePartDialog";

type DialogKind = "param" | "rename" | "delete" | null;

function modesFromTable(design: DesignView, id: string): Record<string, StateAssign> {
  return design.modes.rows.find((r) => r.part === id)?.cells ?? {};
}

/**
 * 1 部品の情報をすべてまとめた詳細（基本・質量・電源・熱・姿勢・通信・配置・接続・モード・診断）。
 * GET /parts/{id} を読み、届くまでは /design の行から同じ形を作って出す。`onClose` があれば見出しの右端に閉じるボタン
 * を出す（一覧のタブの横の欄）。
 */
export function PartDetail({
  design,
  row,
  onSelect,
  onClose,
}: {
  design: DesignView;
  row: PartRow;
  onSelect: (id: string | null) => void;
  onClose?: () => void;
}) {
  const tc = useT().common;
  const t = useT().components.detail;
  const detail = useQuery(q.part(row.id));
  const part = detail.data?.part ?? row;
  // 区分（系）と並びは API（fune_model::builtin）が正本。詳細が届くまでは 1 つの「基本」に並べる
  const basic = design.defs.labels.section.basic;
  const sections = detail.data?.sections ?? [
    {
      key: "basic",
      label: basic?.label ?? "basic",
      label_en: basic?.label_en ?? null,
      attrs: Object.keys(row.values),
    },
  ];
  const modes = detail.data?.modes ?? modesFromTable(design, row.id);
  const diagnostics =
    detail.data?.diagnostics ?? design.diagnostics.filter((d) => d.part === row.id);
  const [dialog, setDialog] = useState<DialogKind>(null);
  const shown = new Set(sections.flatMap((s) => s.attrs));
  const rest = Object.keys(part.values).filter((id) => !shown.has(id));

  return (
    <div className="flex min-h-0 flex-col">
      {/* 系統図のカードと同じ形: サブシステム色の淡い地と左の色の線 */}
      <div
        className={`flex items-start justify-between gap-2 border-line border-b border-l-4 px-4 py-3 ${toneBorder[subsystemTone(part.subsystem, part.role)]}`}
        style={{
          background: `color-mix(in srgb, ${toneHex[subsystemTone(part.subsystem, part.role)]} 10%, var(--color-panel))`,
        }}
      >
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <SubsystemDot group={part.subsystem} role={part.role} />
            <h2 className="truncate font-mono font-semibold text-[15px]">{part.id}</h2>
            {part.count > 1 && <span className="text-ink-3 text-xs">× {part.count}</span>}
          </div>
          <p className="mt-0.5 truncate text-ink-3 text-xs">
            {nodeLabel(design, part.parent)} · {part.kind}
            {part.catalog ? ` · ${part.catalog}` : ""}
          </p>
        </div>
        <div className="flex shrink-0 gap-1">
          <Button size="sm" onClick={() => setDialog("param")}>
            <Plus size={13} />
            {t.addParam}
          </Button>
          <Button
            size="icon-sm"
            variant="outline"
            title={t.rename}
            onClick={() => setDialog("rename")}
          >
            <Pencil size={13} />
          </Button>
          <Button
            size="icon-sm"
            variant="outline"
            title={t.remove}
            onClick={() => setDialog("delete")}
          >
            <Trash2 size={13} />
          </Button>
          {onClose && (
            <Button
              size="icon-sm"
              variant="ghost"
              title={tc.close}
              aria-label={tc.close}
              onClick={onClose}
            >
              <X size={15} />
            </Button>
          )}
        </div>
      </div>
      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-4 py-3">
        {diagnostics.length > 0 && (
          <DetailSection title={t.diagnostics}>
            <ul className="space-y-1">
              {diagnostics.map((d) => (
                <li
                  key={`${d.code}-${d.path}-${d.line}`}
                  className="flex items-start gap-2 text-xs"
                >
                  <SeverityPill severity={d.severity} />
                  <span>
                    {d.path && <span className="font-mono text-ink-3">{d.path} </span>}
                    {d.message}
                  </span>
                </li>
              ))}
            </ul>
          </DetailSection>
        )}
        <DetailSection title={t.basic}>
          <BasicSection design={design} part={part} />
        </DetailSection>
        {sections
          .filter((s) => s.attrs.length > 0)
          .map((s) => (
            <DetailSection key={s.key} title={localized(s.label, s.label_en)}>
              <AttrTable design={design} part={part} attrIds={s.attrs} />
            </DetailSection>
          ))}
        {rest.length > 0 && (
          <DetailSection title={localized(basic?.label ?? "basic", basic?.label_en)}>
            <AttrTable design={design} part={part} attrIds={rest} />
          </DetailSection>
        )}
        <DetailSection title={t.placement}>
          <PlacementSection
            key={`${part.id}-${design.project.revision}`}
            design={design}
            part={part}
          />
        </DetailSection>
        <DetailSection title={t.ports}>
          <PortsSection design={design} part={part} />
        </DetailSection>
        <DetailSection title={t.modes}>
          <PartModes design={design} part={part.id} modes={modes} />
        </DetailSection>
        {detail.data && detail.data.harnesses.length > 0 && (
          <DetailSection title={t.harnesses}>
            <p className="font-mono text-xs">{detail.data.harnesses.join(", ")}</p>
          </DetailSection>
        )}
      </div>
      {dialog === "param" && (
        <AddParameterDialog
          design={design}
          part={part}
          open
          onOpenChange={(o) => !o && setDialog(null)}
        />
      )}
      {dialog === "rename" && (
        <RenamePartDialog
          part={part}
          open
          onOpenChange={(o) => !o && setDialog(null)}
          onRenamed={(id) => onSelect(id)}
        />
      )}
      {dialog === "delete" && (
        <DeletePartDialog
          design={design}
          part={part}
          open
          onOpenChange={(o) => !o && setDialog(null)}
          onDeleted={() => onSelect(null)}
        />
      )}
    </div>
  );
}
