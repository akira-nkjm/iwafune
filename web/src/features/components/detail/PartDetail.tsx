import { useQuery } from "@tanstack/react-query";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { q } from "@/api/queries";
import type { DesignView, PartRow, StateAssign } from "@/api/types";
import { localized, useT } from "@/i18n";
import { DetailAction, DetailHeader, DetailPane } from "@/kit/detail/DetailPane";
import { DetailSection } from "@/kit/detail/DetailSection";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { SystemDot } from "@/kit/SystemDot";
import { DiagnosticItem, diagnosticKey } from "@/kit/status/DiagnosticItem";
import { nodeLabel } from "@/lib/design";
import { systemTone } from "@/lib/systems";
import { Button } from "@/ui/button";
import { DeletePartDialog } from "../DeletePartDialog";
import { AddParameterDialog } from "../parameters/AddParameterDialog";
import { RenamePartDialog } from "../RenamePartDialog";
import { AttrTable } from "./AttrTable";
import { BasicSection } from "./BasicSection";
import { PartModes } from "./PartModes";
import { PlacementSection } from "./PlacementSection";
import { PortsSection } from "./PortsSection";

function modesFromTable(design: DesignView, id: string): Record<string, StateAssign> {
  return design.modes.rows.find((r) => r.part === id)?.cells ?? {};
}

/**
 * 1 部品の情報をすべてまとめた詳細（基本・質量・姿勢・通信・電源・熱・配置・接続・モード・診断）。
 * GET /parts/{id} を読み、届くまでは /design の行から同じ形を作って出す。`onClose` があれば見出しの右端に閉じるボタン
 * を出す（一覧のタブの横の欄）。
 */
export function PartDetail({
  design,
  item,
  onSelect,
  onClose,
}: {
  design: DesignView;
  /** 一覧で選んだ部品（GET /design の行。詳細が届くまではこれで描く） */
  item: PartRow;
  /** 名前を変えたら新しい ID、消したら null で選び直す */
  onSelect: (id: string | null) => void;
  /** 見出しの右端の閉じるボタン（一覧のタブの横の欄だけ） */
  onClose?: () => void;
}) {
  const t = useT().components.detail;
  const detail = useQuery(q.part(item.id));
  const part = detail.data?.part ?? item;
  // 区分（系）と並びは API（fune_model::builtin）が正本。詳細が届くまでは 1 つの「基本」に並べる
  const basic = design.defs.labels.section.basic;
  const sections = detail.data?.sections ?? [
    {
      key: "basic",
      label: basic?.label ?? "basic",
      label_en: basic?.label_en ?? null,
      attrs: Object.keys(item.values),
    },
  ];
  const cells = detail.data?.modes ?? modesFromTable(design, item.id);
  const diagnostics =
    detail.data?.diagnostics ?? design.diagnostics.filter((d) => d.part === item.id);
  const dialogs = useDialogs<"param" | "rename" | "delete">();
  const shown = new Set(sections.flatMap((s) => s.attrs));
  const rest = Object.keys(part.values).filter((id) => !shown.has(id));

  return (
    <DetailPane
      header={
        // 系統図のカードと同じ形: サブシステム色の淡い地と左の色の線
        <DetailHeader
          title={part.id}
          tone={systemTone(part.system)}
          leading={<SystemDot system={part.system} />}
          trailing={part.count > 1 && <span className="text-ink-3 text-xs">× {part.count}</span>}
          subtitle={`${nodeLabel(design, part.system)} · ${part.kind}${part.catalog ? ` · ${part.catalog}` : ""}`}
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
            <AttrTable defs={design.defs} part={part} attrIds={s.attrs} />
          </DetailSection>
        ))}
      {rest.length > 0 && (
        <DetailSection title={localized(basic?.label ?? "basic", basic?.label_en)}>
          <AttrTable defs={design.defs} part={part} attrIds={rest} />
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
        <PartModes table={design.modes} part={part.id} cells={cells} />
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
          table={design.modes}
          part={part}
          onDone={() => onSelect(null)}
        />
      )}
    </DetailPane>
  );
}
