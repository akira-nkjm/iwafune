import { useEnumLabel } from "@/api/labels";
import type { AttrOrigin, Evidence } from "@/api/types";
import { cn } from "@/lib/cn";

const ink: Record<AttrOrigin, string> = {
  catalog: "text-ink-3",
  override: "text-accent-ink",
  kind_default: "text-ink-3",
  derived: "text-pos-ink",
  unset: "text-warn-ink",
};

/** 値の出所（カタログ / 上書き / 既定値 / 導出 / 未設定）と根拠・出典。色だけで区別せず文言を出す（表示名は API の語彙） */
export function OriginBadge({
  origin,
  source,
  evidence,
  className,
}: {
  origin: AttrOrigin;
  source?: string | null;
  evidence?: Evidence | null;
  className?: string;
}) {
  const label = useEnumLabel();
  const weak = evidence === "placeholder" || evidence === "unsourced";
  const originText = label("origin", origin);
  const evidenceText = evidence ? label("evidence", evidence) : null;
  return (
    <span
      className={cn(
        "inline-flex min-w-0 items-center gap-1 overflow-hidden whitespace-nowrap text-[11px]",
        ink[origin],
        className,
      )}
      title={[originText, evidenceText, source].filter(Boolean).join(" · ")}
    >
      {originText}
      {evidenceText && origin !== "unset" && (
        <span className={weak ? "text-warn-ink" : "text-ink-4"}>· {evidenceText}</span>
      )}
      {source && <span className="truncate text-ink-4">· {source}</span>}
    </span>
  );
}
