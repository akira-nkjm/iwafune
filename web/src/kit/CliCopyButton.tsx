import { Terminal } from "lucide-react";
import { toast } from "sonner";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { copyText } from "@/lib/history";

/**
 * 「コマンドをコピー」。GUI の操作と同じことをする `fune` のコマンド（複数なら改行で区切ったすべて）を
 * クリップボードに写す。GUI の操作が CLI でもできることを画面から確かめるためのボタン。
 */
export function CliCopyButton({
  cli,
  className,
  label,
}: {
  cli: readonly string[];
  className?: string;
  label?: string;
}) {
  const t = useT();
  const disabled = cli.length === 0;
  return (
    <button
      type="button"
      disabled={disabled}
      title={disabled ? t.cli.none : cli.join("\n")}
      onClick={async () => {
        const ok = await copyText(cli.join("\n"));
        if (ok) toast.message(t.cli.copied, { description: cli.join("\n") });
        else toast.error(t.cli.copyFailed, { description: cli.join("\n") });
      }}
      className={cn(
        "inline-flex h-7 items-center gap-1.5 rounded-control border border-line bg-panel px-2 text-ink-2 text-xs hover:bg-panel-2 disabled:opacity-50",
        className,
      )}
    >
      <Terminal size={13} />
      {label ?? t.cli.copy}
    </button>
  );
}
