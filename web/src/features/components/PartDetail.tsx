import { useQuery } from "@tanstack/react-query";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { q } from "@/api/queries";
import type { DesignView, PartRow, StateAssign } from "@/api/types";
import { localized, useT } from "@/i18n";
import { DetailAction, DetailHeader, DetailPane } from "@/kit/DetailPane";
import { DetailSection } from "@/kit/DetailSection";
import { DiagnosticItem, diagnosticKey } from "@/kit/DiagnosticItem";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { useDialogs } from "@/kit/use-dialogs";
import { nodeLabel } from "@/lib/design";
import { subsystemTone } from "@/lib/subsystems";
import { Button } from "@/ui/button";
import { AddParameterDialog } from "./AddParameterDialog";
import { AttrTable } from "./AttrTable";
import { BasicSection } from "./BasicSection";
import { DeletePartDialog } from "./DeletePartDialog";
import { PartModes } from "./PartModes";
import { PlacementSection } from "./PlacementSection";
import { PortsSection } from "./PortsSection";
import { RenamePartDialog } from "./RenamePartDialog";

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
  const dialogs = useDialogs<"param" | "rename" | "delete">();
  const shown = new Set(sections.flatMap((s) => s.attrs));
  const rest = Object.keys(part.values).filter((id) => !shown.has(id));

  return (
    <DetailPane
      header={
        // 系統図のカードと同じ形: サブシステム色の淡い地と左の色の線
        <DetailHeader
          title={part.id}
          tone={subsystemTone(part.subsystem, part.role)}
          leading={<SubsystemDot group={part.subsystem} role={part.role} />}
          trailing={part.count > 1 && <span className="text-ink-3 text-xs">× {part.count}</span>}
          subtitle={`${nodeLabel(design, part.parent)} · ${part.kind}${part.catalog ? ` · ${part.catalog}` : ""}`}
          actions={
            <>
              <Button size="sm" onClick={() => dialogs.show("param")}>
                <Plus size={13} />
                {t.addParam}
              </Button>
              <DetailAction icon={Pencil} label={t.rename} onClick={() => dialogs.show("rename")} />
              <DetailAction icon={Trash2} label={t.remove} onClick={() => dialogs.show("delete")} />
            </>
          }
          onClose={onClose}
        />
      }
    >
      {diagnostics.length > 0 && (
        <DetailSection title={t.diagnostics}>
          <ul className="space-y-1">
            {diagnostics.map((d) => (
              <DiagnosticItem key={diagnosticKey(d)} diagnostic={d} />
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
      {dialogs.current === "param" && (
        <AddParameterDialog {...dialogs.props} design={design} part={part} />
      )}
      {dialogs.current === "rename" && (
        <RenamePartDialog {...dialogs.props} part={part} onDone={onSelect} />
      )}
      {dialogs.current === "delete" && (
        <DeletePartDialog
          {...dialogs.props}
          design={design}
          part={part}
          onDeleted={() => onSelect(null)}
        />
      )}
    </DetailPane>
  );
}
