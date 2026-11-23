import { useState } from "react";
import { usePhaseFieldEdit } from "@/api/field-edit";
import { useEnumOptions } from "@/api/labels";
import { useCommandText } from "@/api/ops";
import { useDesign } from "@/api/queries";
import type { CommandText, PhaseView } from "@/api/types";
import { SettingsTable } from "@/features/project/SettingsTable";
import { Attitude } from "@/features/systems/aocs/Attitude";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { Button } from "@/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/ui/dialog";
import {
  addableModes,
  copyOrbitCommand,
  lawKindPath,
  overrideRows,
  unsetLawCommand,
  unsetOrbitCommand,
} from "./override-commands";

/**
 * フェーズの軌道と姿勢則の上書きの編集。軌道は設計の軌道と同じ設定の表（`SettingsTable`）、姿勢則は AOCS のモードと同じ欄
 * （`Attitude`）を、書き先だけ `fune phase set-field` と同じ書き込み（`usePhaseFieldEdit`）にして使う。
 * 足す・消すは 1 手のコマンド（`phase copy-orbit|unset-orbit|unset-law`）と、姿勢則の種類を書く `set-field law.<モード>.law`。
 * フェーズは設計から id で引き直すので、書き込みのあとの再読込で表示が更新される。
 */
export function PhaseOverridesDialog({
  phaseId,
  onOpenChange,
}: {
  phaseId: string;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().phases.overridesDialog;
  const design = useDesign().data;
  const phase = design?.phases.find((p) => p.id === phaseId);
  return (
    <Dialog open onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[85dvh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t.title(phaseId)}</DialogTitle>
          <DialogDescription>{t.description}</DialogDescription>
        </DialogHeader>
        {phase && (
          <div className="grid min-w-0 gap-4">
            <OrbitOverride phase={phase} />
            <LawOverrides phase={phase} modeIds={design?.modes.modes.map((m) => m.id) ?? []} />
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

/** 名詞のコマンドを送る（トーストに変更の要約と「CLI としてコピー」が付く） */
function useSend() {
  const send = useCommandText();
  return {
    pending: send.isPending,
    error: send.error,
    run: (command: CommandText, summary: string) => send.mutate({ command, summary }),
  };
}

export function OrbitOverride({ phase }: { phase: PhaseView }) {
  const t = useT().phases.overridesDialog;
  const edit = usePhaseFieldEdit(phase.id);
  const send = useSend();
  return (
    <Panel>
      <PanelHeader title={t.orbit.title} caption={t.orbit.caption} />
      {phase.orbit_overridden ? (
        <>
          <SettingsTable
            settings={overrideRows(phase.orbit, phase.file)}
            grouped
            edit={edit}
            empty={t.orbit.empty}
          />
          <div className="mt-3 flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={send.pending}
              onClick={() => send.run(copyOrbitCommand(phase.id), t.orbit.copySummary(phase.id))}
            >
              {t.orbit.recopy}
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={send.pending}
              onClick={() => send.run(unsetOrbitCommand(phase.id), t.orbit.removeSummary(phase.id))}
            >
              {t.orbit.remove}
            </Button>
          </div>
        </>
      ) : (
        <div className="grid gap-2">
          <p className="text-ink-3 text-xs">{t.orbit.none}</p>
          <div>
            <Button
              variant="outline"
              size="sm"
              disabled={send.pending}
              onClick={() => send.run(copyOrbitCommand(phase.id), t.orbit.copySummary(phase.id))}
            >
              {t.orbit.copy}
            </Button>
          </div>
        </div>
      )}
      {send.error && (
        <p role="alert" className="mt-2 text-neg-ink text-xs">
          {send.error.message}
        </p>
      )}
    </Panel>
  );
}

export function LawOverrides({ phase, modeIds }: { phase: PhaseView; modeIds: readonly string[] }) {
  const t = useT().phases.overridesDialog;
  const edit = usePhaseFieldEdit(phase.id);
  const send = useSend();
  return (
    <Panel>
      <PanelHeader title={t.law.title} caption={t.law.caption} />
      {phase.laws.length === 0 && <Empty>{t.law.none}</Empty>}
      <div className="grid gap-4">
        {phase.laws.map((l) => (
          <div key={l.mode} className="rounded-control border border-line p-3">
            <div className="mb-2 flex items-center justify-between gap-2">
              <span className="font-medium font-mono text-sm">{l.mode}</span>
              <Button
                variant="ghost"
                size="sm"
                aria-label={t.law.removeOf(l.mode)}
                disabled={send.pending}
                onClick={() =>
                  send.run(unsetLawCommand(phase.id, l.mode), t.law.removeSummary(phase.id, l.mode))
                }
              >
                {t.law.remove}
              </Button>
            </div>
            <Attitude item={l.attitude} edit={edit} />
          </div>
        ))}
      </div>
      <AddLaw
        modes={addableModes(modeIds, phase)}
        onAdd={(mode, kind) => edit.set(lawKindPath(mode), kind)}
        pending={edit.pending}
      />
      {send.error && (
        <p role="alert" className="mt-2 text-neg-ink text-xs">
          {send.error.message}
        </p>
      )}
    </Panel>
  );
}

/** 姿勢則の上書きを足す: モードと種類を選ぶと、種類の欄（`law.<モード>.law`）を書く（あとは軸などの欄を編集） */
function AddLaw({
  modes,
  onAdd,
  pending,
}: {
  modes: readonly string[];
  onAdd: (mode: string, kind: string) => void;
  pending: boolean;
}) {
  const t = useT().phases.overridesDialog.law;
  const kinds = useEnumOptions("attitude_law");
  const [mode, setMode] = useState("");
  const [kind, setKind] = useState("");
  if (modes.length === 0) return <p className="mt-3 text-ink-3 text-xs">{t.noModes}</p>;
  return (
    <div className="mt-3 flex flex-wrap items-center gap-2">
      <NativeSelect
        aria-label={t.mode}
        placeholder={t.mode}
        value={mode}
        onChange={(e) => setMode(e.target.value)}
        options={modes.map((m) => ({ value: m, label: m }))}
      />
      <NativeSelect
        aria-label={t.kind}
        placeholder={t.kind}
        value={kind}
        onChange={(e) => setKind(e.target.value)}
        options={kinds}
      />
      <Button
        variant="outline"
        size="sm"
        disabled={pending || !mode || !kind}
        onClick={() => {
          onAdd(mode, kind);
          setMode("");
          setKind("");
        }}
      >
        {t.add}
      </Button>
    </div>
  );
}
