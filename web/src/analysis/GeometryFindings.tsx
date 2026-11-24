import { useId, useState } from "react";
import type { AnalysisRunResult, Op } from "@/api/types";
import { useT } from "@/i18n";
import { useDialogs } from "@/kit/detail/use-dialogs";
import { Field } from "@/kit/forms/Field";
import { TextInput } from "@/kit/forms/TextInput";
import { Switch } from "@/kit/selection/Switch";
import { Pill } from "@/kit/status/Pill";
import { OpDialog } from "@/ops/OpDialog";
import { Button } from "@/ui/button";
import { findingKey, type GeometryFinding, geometryFindings } from "./geometry-findings";

/**
 * 幾何の所見（箱の重なり・円錐に入る部品）の一覧。承知でない所見は「未承知」、承知済みは理由つきで出し、
 * 承知済みでも所見は消さない。承知の有無・理由・どの形態の承知かは API の出力のまま（ここでは決めない）。
 * `configuration` は結果が使った形態（承知を形態だけに効かせる選択肢に使う。基準なら null）。
 * `onSelectPart` があれば 2 つの部品の名前は部品を選ぶボタンになり（3D）、`actions` があれば承知・取り消しの操作を出す（解析の結果）。
 */
export function GeometryFindings({
  result,
  actions = false,
  onSelectPart,
}: {
  result: AnalysisRunResult;
  actions?: boolean;
  onSelectPart?: (part: string) => void;
}) {
  const t = useT().systems.findings;
  const findings = geometryFindings(result);
  const dialogs = useDialogs<"acknowledge" | "withdraw">();
  const [target, setTarget] = useState<GeometryFinding | null>(null);
  if (findings.length === 0) return null;
  const open = (f: GeometryFinding, action: "acknowledge" | "withdraw") => {
    setTarget(f);
    dialogs.show(action);
  };
  return (
    <section aria-label={t.title} className="space-y-1">
      <h3 className="font-medium text-ink-2 text-xs">{t.title}</h3>
      <ul className="divide-y divide-line-soft rounded-control border border-line">
        {findings.map((f) => (
          <li key={findingKey(f)} className="flex flex-wrap items-center gap-2 px-3 py-1.5 text-xs">
            <FindingRow finding={f} onSelectPart={onSelectPart} />
            {actions && (
              <Button
                size="sm"
                variant="outline"
                onClick={() => open(f, f.acknowledged ? "withdraw" : "acknowledge")}
              >
                {f.acknowledged ? t.withdraw : t.acknowledge}
              </Button>
            )}
          </li>
        ))}
      </ul>
      {dialogs.current === "acknowledge" && target && (
        <AcknowledgeDialog
          {...dialogs.props}
          finding={target}
          configuration={result.configuration ?? null}
        />
      )}
      {dialogs.current === "withdraw" && target && (
        <OpDialog
          {...dialogs.props}
          title={t.withdrawTitle(target.first, target.second)}
          ops={[withdrawOp(target)]}
          summary={t.withdrawSummary(target.first, target.second)}
          confirmLabel={t.withdraw}
        />
      )}
    </section>
  );
}

/** 一覧の 1 行の中身（見出し・種類・状態・理由）。`onSelectPart` があれば 2 つの部品の名前は、その部品を選ぶボタン */
export function FindingRow({
  finding: f,
  onSelectPart,
}: {
  finding: GeometryFinding;
  onSelectPart?: (part: string) => void;
}) {
  const t = useT().systems;
  const part = (id: string) =>
    onSelectPart ? (
      <button
        type="button"
        aria-label={t.layout3d.findings.select(id)}
        className="rounded-chip hover:underline focus-visible:ring-2 focus-visible:ring-accent/40"
        onClick={() => onSelectPart(id)}
      >
        {id}
      </button>
    ) : (
      id
    );
  return (
    <span className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
      <span className="font-medium font-mono text-ink">
        {part(f.first)}
        {f.kind === "cone_clearance" ? " → " : " – "}
        {part(f.second)}
      </span>
      <span className="text-ink-3">{t.findings.kind[f.kind]}</span>
      <Pill tone={f.acknowledged ? "pos" : "warn"}>
        {f.acknowledged ? t.findings.acknowledged : t.findings.open}
      </Pill>
      {f.acknowledged && (
        <span className="min-w-0 text-ink-2">
          {f.reason}
          {f.acknowledgedConfiguration && (
            <span className="ml-1 text-ink-3">
              ({t.findings.scope(f.acknowledgedConfiguration)})
            </span>
          )}
        </span>
      )}
    </span>
  );
}

/** 承知を取り消す Op（承知が形態を指していれば、同じ形態を指す） */
function withdrawOp(f: GeometryFinding): Op {
  return {
    op: "unacknowledge",
    finding: f.kind,
    first: f.first,
    second: f.second,
    configuration: f.acknowledgedConfiguration,
  };
}

/** 理由を入れて承知する。結果が形態の配置のものなら、その形態だけに効かせる選択ができる（既定はすべての配置） */
function AcknowledgeDialog({
  finding: f,
  configuration,
  open,
  onOpenChange,
}: {
  finding: GeometryFinding;
  configuration: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().systems.findings;
  const id = useId();
  const [reason, setReason] = useState("");
  const [only, setOnly] = useState(false);
  const ops: Op[] | null = reason.trim()
    ? [
        {
          op: "acknowledge",
          finding: f.kind,
          first: f.first,
          second: f.second,
          configuration: only ? configuration : null,
          reason: reason.trim(),
        },
      ]
    : null;
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t.acknowledgeTitle(f.first, f.second)}
      description={t.acknowledgeDescription}
      ops={ops}
      summary={t.acknowledgeSummary(f.first, f.second)}
      confirmLabel={t.acknowledge}
    >
      <Field label={t.reason} htmlFor={`${id}-reason`} hint={t.reasonHint}>
        <TextInput
          id={`${id}-reason`}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          autoFocus
        />
      </Field>
      {configuration && (
        <Switch value={only} onChange={setOnly}>
          {t.onlyIn(configuration)}
        </Switch>
      )}
    </OpDialog>
  );
}
