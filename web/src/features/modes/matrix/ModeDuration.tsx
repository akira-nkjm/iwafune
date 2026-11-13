import { Timer } from "lucide-react";
import { useState } from "react";
import { useCommandText } from "@/api/ops";
import type { ModeHeader } from "@/api/types";
import { useT } from "@/i18n";
import { CommitInput } from "@/kit/forms/CommitInput";

/**
 * 列見出しのモードの最長継続時間。ふだんは名前の下の小さな文字で、押すとその場で入力欄になる
 * （Enter か欄を離れると確定、Escape で取り消し。空で確定すると消す）
 */
export function ModeDuration({ mode }: { mode: ModeHeader }) {
  const t = useT().modes;
  const edit = useCommandText();
  const [editing, setEditing] = useState(false);
  const value = mode.max_duration?.display ?? "";
  if (editing) {
    return (
      // biome-ignore lint/a11y/noStaticElementInteractions: 中の欄のフォーカスと Escape を受けて表示に戻すだけ
      <div
        className="mx-auto mt-1 w-24"
        onBlur={() => setEditing(false)}
        onKeyDown={(e) => e.key === "Escape" && setEditing(false)}
      >
        <CommitInput
          ref={(el) => el?.focus()}
          label={t.maxDuration}
          value={value}
          placeholder="15 min"
          className="h-6 text-center text-xs"
          onCommit={(duration) =>
            edit.mutate({
              command: { command: "mode_duration", mode: mode.id, duration: duration || null },
              summary: t.durationSummary(mode.id),
            })
          }
        />
      </div>
    );
  }
  return (
    <button
      type="button"
      title={t.maxDuration}
      aria-label={`${t.maxDuration}: ${value || t.durationUnset}`}
      onClick={() => setEditing(true)}
      className={`mx-auto mt-1 flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] hover:bg-panel hover:text-ink ${value ? "text-ink-3" : "text-ink-4"}`}
    >
      <Timer size={11} aria-hidden="true" />
      <span className="num">{value ? t.durationAtMost(value) : t.durationAdd}</span>
    </button>
  );
}
