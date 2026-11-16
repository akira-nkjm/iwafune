import { useT } from "@/i18n";
import { Pill } from "@/kit/status/Pill";
import { Button } from "@/ui/button";

/**
 * 見出しの「共有カタログ」ボタン。取得が要る共有カタログがあれば警告のピルに数を出し、
 * 共有カタログがすべて読み込めていれば数だけを控えめに出す（色だけで伝えず、読み上げ用の名前にも数を入れる）。
 */
export function SharedCatalogsButton({
  total,
  needed,
  onClick,
}: {
  /** 共有カタログの数 */
  total: number;
  /** 取得が要る数 */
  needed: number;
  onClick: () => void;
}) {
  const m = useT().catalog.shared;
  return (
    <Button
      variant="outline"
      onClick={onClick}
      aria-label={needed > 0 ? `${m.title}: ${m.neededCount(needed)}` : undefined}
    >
      {m.title}
      {needed > 0 ? (
        <Pill tone="warn" title={m.neededCount(needed)}>
          {needed}
        </Pill>
      ) : (
        total > 0 && <span className="text-ink-3 text-xs">{total}</span>
      )}
    </Button>
  );
}
