import { Link } from "react-router";
import type { DiagnosticView } from "@/api/types";
import { cn } from "@/lib/cn";
import { Panel } from "../panel/Panel";
import { SeverityPill } from "./SeverityPill";

/** 一覧で 1 件を見分けるキー（同じ規則・同じ対象・同じ位置・同じ本文の診断は同じもの）。診断を並べる所はこれを使う */
export function diagnosticKey(d: DiagnosticView): string {
  return [d.code, d.path, d.file, d.line, d.column, d.message].join("\u0000");
}

/** 該当するファイルと行（`parts/x.toml:12`。`column` があれば `:12:5`）。ファイルが無ければ null */
function placeText(d: DiagnosticView, column: boolean): string | null {
  if (!d.file) return null;
  if (!d.line) return d.file;
  return column && d.column ? `${d.file}:${d.line}:${d.column}` : `${d.file}:${d.line}`;
}

/** 対象の表示用のパス。`to`（直しに行くページ。`lib/target-link.ts` の `targetLink`）があればリンクにする */
function TargetPath({
  d,
  to,
  className,
}: {
  d: DiagnosticView;
  to?: string | null;
  className?: string;
}) {
  if (!d.path) return null;
  if (to)
    return (
      <Link to={to} className={cn("font-mono text-accent-ink hover:underline", className)}>
        {d.path}
      </Link>
    );
  return <span className={cn("font-mono text-ink-3", className)}>{d.path}</span>;
}

/**
 * 診断 1 件の行（重さ・対象のパス・本文・ファイルの行）。`<ul>` に並べる。`to` を渡すとパスが直しに行くページへの
 * リンクになる（呼び手が `targetLink` で作る。いまのページの対象なら渡さない）
 */
export function DiagnosticItem({
  diagnostic: d,
  to,
}: {
  diagnostic: DiagnosticView;
  to?: string | null;
}) {
  const place = placeText(d, false);
  return (
    <li className="flex items-start gap-2 text-xs">
      <SeverityPill severity={d.severity} />
      <span className="min-w-0">
        <TargetPath d={d} to={to} className="mr-1" />
        {d.message}
        {place && <span className="ml-1 font-mono text-ink-4">{place}</span>}
      </span>
    </li>
  );
}

/** 診断 1 件のカード（重さ・コード・対象のパス・ファイルの行と列・本文・該当行の前後の本文）。診断のページの一覧 */
export function DiagnosticCard({
  diagnostic: d,
  to,
}: {
  diagnostic: DiagnosticView;
  to?: string | null;
}) {
  const place = placeText(d, true);
  return (
    <Panel className="space-y-1.5 py-3">
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <SeverityPill severity={d.severity} />
        <span className="font-mono text-ink-3">{d.code}</span>
        <TargetPath d={d} to={to} />
        {place && <span className="ml-auto font-mono text-ink-4">{place}</span>}
      </div>
      <p className="text-ink text-md">{d.message}</p>
      {d.excerpt && (
        <pre className="overflow-x-auto rounded-control bg-panel-2 px-3 py-2 font-mono text-[11.5px] text-ink-2">
          {d.excerpt.text.split("\n").map((line, i) => {
            const n = (d.excerpt?.first_line ?? 1) + i;
            return (
              <div key={n} className={n === d.line ? "bg-warn-bg" : undefined}>
                <span className="mr-3 inline-block w-8 select-none text-right text-ink-4">{n}</span>
                {line}
              </div>
            );
          })}
        </pre>
      )}
    </Panel>
  );
}
