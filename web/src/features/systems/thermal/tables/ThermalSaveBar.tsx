import { Save, Undo2 } from "lucide-react";
import { useApplyOps } from "@/api/ops";
import type { Op, ThermalNodeTemperature } from "@/api/types";
import { useT } from "@/i18n";
import { OpPreview } from "@/ops/OpPreview";
import { useOpCheck } from "@/ops/use-op-check";
import { Button } from "@/ui/button";
import { verdictText } from "../RangeTip";

/** 変わった節点だけ並べる上限（残りは件数で示す） */
const SHOWN = 6;

/** 下書きの前後で表示の変わった節点（表示文字列の比較。値は API が整形したもの） */
function shifted(
  before: readonly ThermalNodeTemperature[],
  after: readonly ThermalNodeTemperature[],
) {
  return after.flatMap((a) => {
    const b = before.find((x) => x.node === a.node);
    return b && (b.min.display !== a.min.display || b.max.display !== a.max.display)
      ? [{ node: a.node, before: b, after: a }]
      : [];
  });
}

/**
 * 節点の温度への影響: 保存済みの設計と、下書きを当てた設計の最低・最高（全ケース）を、変わった節点だけ前 → 後で並べる。
 * 値と判定は API の整形のまま（ここでは計算しない）。
 */
function Effect({
  before,
  after,
  working,
}: {
  before: readonly ThermalNodeTemperature[];
  after: readonly ThermalNodeTemperature[];
  working: boolean;
}) {
  const t = useT().systems.thermalNet;
  const rows = shifted(before, after);
  return (
    <div className="space-y-1" aria-live="polite">
      <p className="font-medium text-ink-2 text-xs">{t.effectTitle}</p>
      {rows.length === 0 ? (
        <p className="text-[11.5px] text-ink-3">{working ? t.effectWorking : t.effectNone}</p>
      ) : (
        <ul className="grid gap-x-4 gap-y-0.5 text-[11.5px] sm:grid-cols-2 xl:grid-cols-3">
          {rows.slice(0, SHOWN).map((r) => (
            <li key={r.node} className="flex flex-wrap items-baseline gap-x-2">
              <span className="font-mono text-ink-2">{r.node}</span>
              <span className="num text-ink-3">
                {t.effectMin} {r.before.min.display} → {r.after.min.display}
              </span>
              <span className={`num ${verdictText[r.after.verdict]}`}>
                {t.effectMax} {r.before.max.display} → {r.after.max.display}
              </span>
            </li>
          ))}
        </ul>
      )}
      {rows.length > SHOWN && (
        <p className="text-[11px] text-ink-3">{t.effectMore(rows.length - SHOWN)}</p>
      )}
    </div>
  );
}

/**
 * 下書きの保存の帯（モードの表の `ModesSaveBar` と同じ作り）: 同じ操作の CLI（dry_run。通らなければ保存させず、検査の理由を出す）、
 * 件数、節点の温度への影響（保存の前）、破棄・保存。
 */
export function ThermalSaveBar({
  ops,
  error,
  count,
  before,
  after,
  working,
  onDone,
}: {
  /** 下書きを service が Op にしたもの（読んでいる間と読めないときは null） */
  ops: Op[] | null;
  /** 下書きを Op にできなかった理由 */
  error: string | null;
  count: number;
  before: readonly ThermalNodeTemperature[];
  after: readonly ThermalNodeTemperature[];
  /** 下書きを当てた温度を計算している最中 */
  working: boolean;
  onDone: () => void;
}) {
  const t = useT();
  const apply = useApplyOps();
  const check = useOpCheck(ops);
  const save = () => {
    if (ops && check.ok)
      apply.mutate(
        { ops, summary: t.systems.thermalNet.saveSummary(count) },
        { onSuccess: onDone },
      );
  };
  return (
    <div className="flex items-start gap-4 border-line border-t bg-panel-2 px-4 py-2.5">
      <div className="min-w-0 flex-1 space-y-2">
        <OpPreview ops={ops} compact />
        {/* 検査が弾いた下書きは、温度を当てられない（理由は上の帯が出す） */}
        {!check.failed && <Effect before={before} after={after} working={working} />}
        <p className="truncate text-[11px] text-ink-3">{t.systems.thermalNet.draftNote}</p>
      </div>
      <div className="flex shrink-0 flex-col items-end gap-1.5">
        <span className="text-ink-2 text-xs">{t.systems.thermalNet.changed(count)}</span>
        {error && <span className="max-w-80 text-[11px] text-neg-ink">{error}</span>}
        <div className="flex gap-2">
          <Button variant="outline" onClick={onDone}>
            <Undo2 size={14} />
            {t.common.discard}
          </Button>
          <Button onClick={save} disabled={!check.ok || apply.isPending}>
            <Save size={14} />
            {apply.isPending ? t.common.saving : t.common.save}
          </Button>
        </div>
      </div>
    </div>
  );
}
