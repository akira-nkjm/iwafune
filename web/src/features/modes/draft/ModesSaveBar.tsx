import { Save, Undo2 } from "lucide-react";
import { useApplyOps } from "@/api/ops";
import type { Op } from "@/api/types";
import { useT } from "@/i18n";
import { OpPreview } from "@/ops/OpPreview";
import { useOpCheck } from "@/ops/use-op-check";
import { Button } from "@/ui/button";

/**
 * 下書きの保存の帯: 同じ操作の CLI（dry_run）、件数、破棄・保存。dry_run が通らなければ保存させない。
 * 表の電力・合計・余裕はこの下書きを当てた値（まだ保存していない）なので、そのことも帯に書いておく。
 */
export function ModesSaveBar({
  ops,
  error,
  count,
  onDone,
}: {
  /** 下書きを service が Op にしたもの（読んでいる間と読めないときは null） */
  ops: Op[] | null;
  /** 下書きを Op にできなかった理由（打った時間割合が数でないなど） */
  error: string | null;
  count: number;
  onDone: () => void;
}) {
  const t = useT();
  const apply = useApplyOps();
  // 時間割合の合計などの規則はサーバー（dry_run）が検査する
  const check = useOpCheck(ops);
  // サーバー（dry_run）が弾いた理由は上のプレビューに出る。ここには下書きの時点で分かる誤りだけを出す
  const reason = error;
  const save = () => {
    if (ops && check.ok)
      apply.mutate({ ops, summary: t.modes.saveSummary(count) }, { onSuccess: onDone });
  };
  return (
    <div className="flex items-center gap-4 border-line border-t bg-panel-2 px-4 py-2.5">
      <div className="min-w-0 flex-1 space-y-1">
        <OpPreview ops={ops} compact />
        <p className="truncate text-[11px] text-ink-3">{t.modes.draftApplied}</p>
      </div>
      <div className="flex shrink-0 flex-col items-end gap-1.5">
        <span className="text-ink-2 text-xs">{t.modes.changed(count)}</span>
        {reason && <span className="max-w-80 text-[11px] text-neg-ink">{reason}</span>}
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
