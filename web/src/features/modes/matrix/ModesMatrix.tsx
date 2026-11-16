import { Plus } from "lucide-react";
import { useState } from "react";
import { MissingList } from "@/analysis/MissingList";
import type { DesignView, System } from "@/api/types";
import { useT } from "@/i18n";
import { Reveal } from "@/kit/Reveal";
import { Button } from "@/ui/button";
import { draftCount, type ModeDraft, withCell } from "../draft/draft";
import { ModesSaveBar } from "../draft/ModesSaveBar";
import { useModeDraftLoad } from "../draft/use-mode-draft-load";
import { ModesTable } from "./ModesTable";
import { matrixGroups } from "./matrix-rows";

/**
 * モード表のカード: 見出しの帯（凡例・モード追加）、行列、下書きの保存の帯。下書きを当てた計算は `useModeDraftLoad`、
 * 行への変換は `matrixGroups`、表示は `ModesTable`。カードは画面の高さまで伸び、表はカードの中で縦横にスクロールする。
 */
export function ModesMatrix({
  design,
  draft,
  onDraft,
  onAdd,
  system,
}: {
  design: DesignView;
  draft: ModeDraft;
  onDraft: (draft: ModeDraft) => void;
  onAdd?: () => void;
  system?: System;
}) {
  const tm = useT().modes;
  const [eclipse, setEclipse] = useState(false);
  const table = design.modes;
  const computed = useModeDraftLoad(table, draft, system);
  const count = draftCount(draft);
  return (
    <div className="flex min-h-0 flex-1 flex-col p-6">
      <section
        aria-label={tm.matrixTitle}
        className="flex min-h-0 flex-col overflow-hidden rounded-card border border-line bg-panel"
      >
        <div className="flex shrink-0 flex-wrap items-center gap-x-4 gap-y-2 border-line border-b px-4 py-3">
          <h2 className="font-semibold text-sm">{tm.matrixTitle}</h2>
          <LightingSwitch value={eclipse} onChange={setEclipse} />
          <p className="text-xs text-ink-3">{tm.hint}</p>
          <div className="ml-auto flex gap-3 whitespace-nowrap text-xs text-ink-3">
            <span>{tm.legendOn}</span>
            <span>{tm.legendDuty}</span>
            <span>{tm.legendOff}</span>
          </div>
          {onAdd && (
            <Button variant="outline" onClick={onAdd}>
              <Plus size={14} />
              {tm.add}
            </Button>
          )}
        </div>
        {computed.error && (
          <p role="alert" className="p-3 text-neg-ink">
            {computed.error}
          </p>
        )}
        {computed.view.data && computed.view.data.missing.length > 0 && (
          <div className="shrink-0 p-3">
            <MissingList design={design} missing={computed.view.data.missing} />
          </div>
        )}
        <div className="min-h-0 flex-1 scroll-pt-16 scroll-pb-36 scroll-pl-52 overflow-auto">
          <ModesTable
            key={eclipse ? "eclipse" : "sunlit"}
            modes={table.modes}
            groups={matrixGroups(design, draft, computed.view.data, tm, eclipse, system)}
            eclipse={eclipse}
            view={computed.view}
            system={system}
            updating={computed.view.updating}
            onCell={({ mode, part }, cell) =>
              onDraft(withCell(table, draft, mode, part, cell, eclipse))
            }
          />
        </div>
        {/* 保存の帯はカードのいちばん下。最初の下書きで滑らかに開き、保存・破棄で滑らかに閉じる */}
        <Reveal open={count > 0}>
          <ModesSaveBar
            ops={computed.save.ops}
            error={computed.save.error}
            count={count}
            onDone={() => onDraft(new Map())}
          />
        </Reveal>
      </section>
    </div>
  );
}

/** 日照／蝕の表示選択。ネイティブのボタンで Tab・Enter・Space を使える。 */
export function LightingSwitch({
  value,
  onChange,
}: {
  value: boolean;
  onChange: (eclipse: boolean) => void;
}) {
  const t = useT().modes;
  return (
    <fieldset aria-label={t.lighting} className="flex gap-1">
      <Button
        variant="outline"
        aria-pressed={!value}
        onClick={() => onChange(false)}
        className="aria-pressed:border-accent aria-pressed:bg-accent/15 aria-pressed:text-accent"
      >
        {t.sunlit}
      </Button>
      <Button
        variant="outline"
        aria-pressed={value}
        onClick={() => onChange(true)}
        className="aria-pressed:border-accent aria-pressed:bg-accent/15 aria-pressed:text-accent"
      >
        {t.eclipseView}
      </Button>
    </fieldset>
  );
}

/** 系のタブも定義ページと同じ下書き・保存・セルを使う。 */
export function SystemModes({ design, system }: { design: DesignView; system: System }) {
  const [draft, setDraft] = useState<ModeDraft>(new Map());
  return <ModesMatrix design={design} system={system} draft={draft} onDraft={setDraft} />;
}
