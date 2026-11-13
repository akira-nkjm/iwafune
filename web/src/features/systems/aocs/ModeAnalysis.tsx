import type { AocsAnalysisState, AocsModeView } from "@/api/types";
import { useT } from "@/i18n";
import { Pill } from "@/kit/status/Pill";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { quantityText } from "@/quantity/format";

/** Rust が返した解析値と状態。判定の集約や数の計算を表示側でしない。 */
export function ModeAnalysis({ item }: { item: AocsModeView }) {
  const t = useT().systems.aocsModes;
  return (
    <section className="space-y-3 border-line border-t pt-3">
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
        <p className="mt-1 font-mono text-xs">{quantityText(item.mtq_unload_time)}</p>
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
        {state.status === "ok" && state.verdict ? (
          <VerdictPill verdict={state.verdict} />
        ) : (
          <Pill tone={state.status === "ok" ? "neutral" : "warn"}>
            {t.systems.status[state.status]}
          </Pill>
        )}
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
