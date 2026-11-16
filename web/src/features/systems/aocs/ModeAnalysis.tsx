import type { ReactNode } from "react";
import type { AocsAnalysisState, AocsModeView, QuantityView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Pill } from "@/kit/status/Pill";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { quantityText } from "@/quantity/format";

/**
 * カードに出す 3 つの値（運動量の蓄積 / 周回・アンロード / 日・視野の判定）。解析が通っていなければ値の代わりに状態を出す。
 */
export function ModeValues({ item }: { item: AocsModeView }) {
  const t = useT().systems.aocsModes;
  return (
    <dl className="grid gap-1.5 text-xs">
      <ValueRow label={t.perOrbitShort} title={t.perOrbit}>
        <QuantityOrState value={item.momentum_per_orbit} state={item.rw} />
      </ValueRow>
      <ValueRow label={t.perDayShort} title={t.perDay}>
        <QuantityOrState value={item.unloads_per_day} state={item.rw} />
      </ValueRow>
      <ValueRow label={t.fov} title={t.sensorFov}>
        <StatePill state={item.sensor_fov} />
      </ValueRow>
    </dl>
  );
}

function ValueRow({
  label,
  title,
  children,
}: {
  label: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <dt className="min-w-0 truncate text-ink-3" title={title}>
        {label}
      </dt>
      <dd className="shrink-0">{children}</dd>
    </div>
  );
}

function QuantityOrState({
  value,
  state,
}: {
  value: QuantityView | null;
  state: AocsAnalysisState;
}) {
  return state.status === "ok" ? (
    <span className="num font-medium">{quantityText(value)}</span>
  ) : (
    <StatePill state={state} />
  );
}

/** 解析の判定（通っていれば判定、通っていなければ状態）のピル */
function StatePill({ state }: { state: AocsAnalysisState }) {
  const t = useT();
  return state.status === "ok" && state.verdict ? (
    <VerdictPill verdict={state.verdict} />
  ) : (
    <Pill tone={state.status === "ok" ? "neutral" : "warn"}>{t.systems.status[state.status]}</Pill>
  );
}

/** Rust が返した解析値と状態。判定の集約や数の計算を表示側でしない。 */
export function ModeAnalysis({ item }: { item: AocsModeView }) {
  const t = useT().systems.aocsModes;
  return (
    <section className="space-y-3 border-line border-t py-3">
      <div>
        <AnalysisState title={t.momentum} state={item.rw} />
        <dl className="mt-2 grid grid-cols-2 gap-2 text-xs">
          <div>
            <dt className="text-ink-3">{t.perOrbit}</dt>
            <dd className="mt-1 font-mono">{quantityText(item.momentum_per_orbit)}</dd>
          </div>
          <div>
            <dt className="text-ink-3">{t.perDay}</dt>
            <dd className="mt-1 font-mono">{quantityText(item.unloads_per_day)}</dd>
          </div>
        </dl>
      </div>
      <div>
        <AnalysisState title={t.unloadTime} state={item.unloading} />
        {item.unload_times.length === 0 ? (
          <p className="mt-1 font-mono text-xs">{quantityText(null)}</p>
        ) : (
          <ul className="mt-1 space-y-0.5 font-mono text-xs">
            {item.unload_times.map((u) => (
              <li key={u.means}>
                <span className="text-ink-3">{localized(u.label, u.label_en)}</span>{" "}
                {quantityText(u.time)}
              </li>
            ))}
          </ul>
        )}
      </div>
      <AnalysisState title={t.sensorFov} state={item.sensor_fov} />
    </section>
  );
}

function AnalysisState({ title, state }: { title: string; state: AocsAnalysisState }) {
  const t = useT();
  return (
    <div>
      <div className="flex items-center justify-between gap-2 text-sm">
        <h3 className="font-medium">{title}</h3>
        <StatePill state={state} />
      </div>
      {state.missing.length > 0 && (
        <details className="mt-1 text-xs text-ink-3">
          <summary>{t.systems.missing(state.missing.length)}</summary>
          <ul>
            {state.missing.map((m) => (
              <li key={m.path} className="font-mono">
                {m.path}
              </li>
            ))}
          </ul>
        </details>
      )}
      {state.error && (
        <p role="alert" className="mt-1 text-xs text-neg-ink">
          {state.error}
        </p>
      )}
    </div>
  );
}
