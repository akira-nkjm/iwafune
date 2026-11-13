import { ExternalLink as ExternalIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/**
 * 外の Web ページ（データシートなど）を新しいタブで開くリンク。URL は API が Web の URL と判定したものだけを渡す
 * （`SourceView.link`・`CatalogItemView.datasheet_link`）。中身が無ければ URL を出す
 */
export function ExternalLink({
  href,
  label,
  children,
  className,
}: {
  href: string;
  /** 読み上げとツールチップの名前（「開く」など） */
  label?: string;
  children?: ReactNode;
  className?: string;
}) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      title={label ? `${label}: ${href}` : href}
      className={cn(
        "inline-flex min-w-0 items-center gap-1 text-accent-ink hover:underline",
        className,
      )}
    >
      <span className="truncate">{children ?? href}</span>
      <ExternalIcon size={12} className="shrink-0" aria-hidden />
    </a>
  );
}
