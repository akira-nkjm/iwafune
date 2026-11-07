import { RotateCcw } from "lucide-react";

/** 値を消して元（カタログ・既定）に戻すボタン（表の行の右端） */
export function ResetButton({ title, onClick }: { title: string; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      aria-label={title}
      className="inline-flex size-6 items-center justify-center rounded-[6px] text-ink-3 hover:bg-panel-2 hover:text-ink"
    >
      <RotateCcw size={12} />
    </button>
  );
}
