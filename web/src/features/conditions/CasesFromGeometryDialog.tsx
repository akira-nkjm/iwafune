import { CalendarClock } from "lucide-react";
import { useState } from "react";
import { useGeometryCaseOps } from "@/api/ops";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { OpDialog } from "@/ops/OpDialog";
import { Button } from "@/ui/button";
import { Input } from "@/ui/input";

/** 解析ケースのページの操作: 最悪の日からケースを作る（`fune case from-geometry`）。設計を読み込んでから押せる */
export function CasesFromGeometryButton({ design }: { design: DesignView | undefined }) {
  const m = useT().conditions.fromGeometry;
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button size="sm" disabled={!design} onClick={() => setOpen(true)}>
        <CalendarClock size={13} />
        {m.open}
      </Button>
      {open && design && <CasesFromGeometryDialog design={design} onClose={() => setOpen(false)} />}
    </>
  );
}

/**
 * 最悪の日からのケース作成。Op の列は service（`POST /ops/cases_from_geometry`）が作り、プレビューと保存は共通のダイアログに任せる。
 * フェーズの選択肢とモードの選択肢は設計から（フェーズがある設計ではフェーズが必須という規則は service が持ち、足りなければ欄の下に出る）。
 */
function CasesFromGeometryDialog({ design, onClose }: { design: DesignView; onClose: () => void }) {
  const m = useT().conditions;
  const t = m.fromGeometry;
  // フェーズがある設計では、先頭のフェーズを初めの選択にする（選び直せる）
  const [phase, setPhase] = useState(design.phases[0]?.id ?? "");
  const [prefix, setPrefix] = useState("");
  const [mode, setMode] = useState("");
  const built = useGeometryCaseOps({
    phase: phase || null,
    prefix: prefix.trim() || null,
    mode: mode || null,
  });
  return (
    <OpDialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={t.title}
      description={t.description}
      ops={built.ops}
      summary={t.summary(phase)}
      confirmLabel={t.confirm}
      wide
    >
      {design.phases.length > 0 && (
        <Field label={t.phase} htmlFor="geometry-phase">
          <NativeSelect
            id="geometry-phase"
            value={phase}
            options={design.phases.map((p) => ({ value: p.id, label: `${p.id} — ${p.label}` }))}
            onChange={(e) => setPhase(e.target.value)}
          />
        </Field>
      )}
      <Field label={t.prefix} htmlFor="geometry-prefix" hint={t.prefixHint}>
        <Input
          id="geometry-prefix"
          className="font-mono"
          value={prefix}
          onChange={(e) => setPrefix(e.target.value)}
        />
      </Field>
      <Field label={t.mode} htmlFor="geometry-mode">
        <NativeSelect
          id="geometry-mode"
          value={mode}
          placeholder={t.noMode}
          options={design.modes.modes.map((x) => ({ value: x.id, label: x.label }))}
          onChange={(e) => setMode(e.target.value)}
        />
      </Field>
      {built.error && <p className="text-[11px] text-neg-ink">{built.error}</p>}
    </OpDialog>
  );
}
