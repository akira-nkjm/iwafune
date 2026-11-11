import type { Op } from "@/api/types";
import { useT } from "@/i18n";
import { CliCopyButton } from "@/kit/CliCopyButton";
import { cn } from "@/lib/cn";
import { withNote } from "@/lib/design";
import { useOpCheck } from "./use-op-check";

/**
 * 確定前のプレビュー: dry_run で同じ操作の CLI コマンドと変わるファイルを出す。弾かれたら理由を出す。
 * GUI でできることは CLI でもできることを、操作の前に確かめられるようにする。
 * 問い合わせ中は前の結果を薄くして出しておく。`compact` はコマンドを 1 行にまとめる（編集のたびに出る帯で、
 * コマンドの数が変わっても高さが変わらないように。2 つ目からは「ほか N 件」とし、全文はコピーと title で見る）。
 */
export function OpPreview({ ops, compact = false }: { ops: Op[] | null; compact?: boolean }) {
  const t = useT();
  const check = useOpCheck(ops);
  if (!ops || ops.length === 0) return null;
  const cli = check.result?.cli ?? [];
  const changed = check.result?.changed ?? [];
  const waiting = check.pending && cli.length === 0;
  return (
    <div
      className={cn(
        "min-w-0 rounded-control border border-line bg-panel-2 transition-opacity",
        compact ? "space-y-1 px-2.5 py-2" : "space-y-1.5 p-2.5",
        check.pending && cli.length > 0 && "opacity-60",
      )}
    >
      {compact ? (
        <div className="flex items-center gap-2">
          <span className="shrink-0 font-medium text-ink-2 text-xs">{t.cli.sameOp}</span>
          {check.error ? (
            <span className="min-w-0 flex-1 truncate text-[11.5px] text-neg-ink">
              {check.error}
            </span>
          ) : (
            <code
              title={cli.join("\n")}
              className="min-w-0 flex-1 truncate font-mono text-[11.5px] text-ink-2"
            >
              {waiting ? t.cli.checking : (cli[0] ?? "—")}
            </code>
          )}
          {cli.length > 1 && (
            <span className="shrink-0 text-[11px] text-ink-3">{t.cli.more(cli.length - 1)}</span>
          )}
          <CliCopyButton cli={cli} className="shrink-0" />
        </div>
      ) : (
        <>
          <div className="flex items-center justify-between gap-2">
            <span className="font-medium text-ink-2 text-xs">{t.cli.sameOp}</span>
            <CliCopyButton cli={cli} />
          </div>
          {check.error ? (
            <p className="text-[11.5px] text-neg-ink">{check.error}</p>
          ) : (
            <pre className="max-h-28 overflow-auto whitespace-pre-wrap font-mono text-[11.5px] text-ink-2">
              {waiting ? t.cli.checking : cli.join("\n") || "—"}
            </pre>
          )}
        </>
      )}
      {changed.length > 0 ? (
        <details className="text-[11px] text-ink-3">
          <summary className="cursor-pointer truncate">
            {t.cli.changedFiles}{" "}
            {changed.map((c) => withNote(c.path, t.cli.change[c.change])).join(", ")}
          </summary>
          <pre className="mt-1 max-h-40 overflow-auto whitespace-pre font-mono text-[11px] text-ink-2">
            {changed.map((c) => c.diff).join("\n")}
          </pre>
        </details>
      ) : (
        // 1 行にまとめるときは、結果が来る前から「変わるファイル」の 1 行ぶんを空けておく
        compact && (
          <p aria-hidden="true" className="text-[11px]">
            {" "}
          </p>
        )
      )}
      {check.result?.diagnostics
        .filter((d) => d.severity !== "info")
        .map((d) => (
          <p key={`${d.code}-${d.path}`} className="text-[11px] text-warn-ink">
            {d.path ? `${d.path}: ` : ""}
            {d.message}
          </p>
        ))}
    </div>
  );
}
