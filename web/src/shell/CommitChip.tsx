import { useQuery } from "@tanstack/react-query";
import { GitBranch, History, Redo2, Undo2 } from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "react-router";
import { q } from "@/api/queries";
import { useLang, useT } from "@/i18n";
import { CliCopyButton } from "@/kit/CliCopyButton";
import { useChangeHistory } from "@/lib/history";
import { paths } from "@/lib/nav";
import { Popover, PopoverContent, PopoverTrigger } from "@/ui/popover";

/** ナビのアイコンと同じ幅（16 px）の枠。小さいアイコンも中央に置いて、左右の空きをナビの行と同じにする */
function IconSlot({ children }: { children: ReactNode }) {
  return <span className="flex w-4 shrink-0 justify-center">{children}</span>;
}

/** サイドバー下部: ブランチ・revision・診断の件数・このセッションの変更履歴（CLI としてコピー） */
export function CommitChip({ rail = false }: { rail?: boolean }) {
  const project = useQuery(q.project());
  const t = useT();
  const lang = useLang();
  const history = useChangeHistory();
  const p = project.data;
  const errors = p?.diagnostics.errors ?? 0;
  const warnings = p?.diagnostics.warnings ?? 0;
  if (rail)
    return (
      <Link
        to={paths.diagnostics()}
        title={t.shell.diagTitle(errors, warnings)}
        aria-label={t.shell.diagLabel(errors, warnings)}
        className="mx-auto flex size-8 flex-col items-center justify-center rounded-control text-[10px] hover:bg-panel"
      >
        <GitBranch size={14} className="text-ink-3" />
        <span
          className={errors > 0 ? "text-neg-ink" : warnings > 0 ? "text-warn-ink" : "text-ink-4"}
        >
          {errors > 0 ? errors : warnings}
        </span>
      </Link>
    );
  return (
    <div className="space-y-2 px-1">
      <div className="flex items-center gap-3 px-3 text-[11.5px] text-ink-3">
        <IconSlot>
          <GitBranch size={13} />
        </IconSlot>
        <span className="truncate font-mono">{p?.branch ?? "—"}</span>
        <span className="ml-auto font-mono text-ink-4" title={p?.revision}>
          {p?.revision.slice(0, 7) ?? ""}
        </span>
      </div>
      {project.isError ? (
        <div className="rounded-chip bg-neg-bg px-2 py-1 text-[11px] text-neg-ink">
          {t.shell.offline}
        </div>
      ) : (
        <Link
          to={paths.diagnostics()}
          className="flex items-center gap-2 rounded-chip py-0.5 pr-1 pl-10 text-[11.5px] hover:bg-panel"
        >
          <span className={errors > 0 ? "text-neg-ink" : "text-ink-3"}>
            {t.shell.errors(errors)}
          </span>
          <span className={warnings > 0 ? "text-warn-ink" : "text-ink-3"}>
            {t.shell.warnings(warnings)}
          </span>
        </Link>
      )}
      <Popover>
        <PopoverTrigger asChild>
          <button
            type="button"
            className="flex w-full items-center gap-3 rounded-chip px-3 py-0.5 text-[11.5px] text-ink-3 hover:bg-panel"
          >
            <IconSlot>
              <History size={13} />
            </IconSlot>
            {t.shell.history}
            <span className="num ml-auto">{history.length}</span>
          </button>
        </PopoverTrigger>
        <PopoverContent side="right" align="end" className="w-[420px] p-0">
          <div className="border-line border-b px-3 py-2 font-medium text-sm">
            {t.shell.historyTitle}
          </div>
          <ul className="max-h-[360px] overflow-y-auto">
            {history.length === 0 && (
              <li className="px-3 py-4 text-ink-3 text-xs">{t.shell.historyEmpty}</li>
            )}
            {history.map((h) => (
              <li key={h.id} className="space-y-1 border-line-soft border-b px-3 py-2">
                <div className="flex items-center gap-2 text-xs">
                  {h.kind === "undo" && <Undo2 size={12} className="shrink-0 text-ink-3" />}
                  {h.kind === "redo" && <Redo2 size={12} className="shrink-0 text-ink-3" />}
                  <span className="font-medium">{h.summary}</span>
                  <span className="ml-auto text-ink-4">{h.at.toLocaleTimeString(lang)}</span>
                </div>
                {h.cli.length > 0 && (
                  <pre className="overflow-x-auto rounded-chip bg-panel-2 px-2 py-1 font-mono text-[11px] text-ink-2">
                    {h.cli.join("\n")}
                  </pre>
                )}
                {h.cli.length > 0 && <CliCopyButton cli={h.cli} />}
              </li>
            ))}
          </ul>
        </PopoverContent>
      </Popover>
    </div>
  );
}
