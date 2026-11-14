import type { LucideIcon } from "lucide-react";
import { X } from "lucide-react";
import type { ReactNode } from "react";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { type Tone, toneBorder, toneTint } from "@/lib/tone";
import { Button } from "@/ui/button";

/**
 * 1 つのものの詳細の枠（一覧の横の欄・詳細のモーダルの中身）。上に見出し（`header`。ふつうは `DetailHeader`）、
 * 残りの高さに縦に流れる中身（区分は `DetailSection`）
 */
export function DetailPane({ header, children }: { header: ReactNode; children: ReactNode }) {
  return (
    <div className="flex min-h-0 flex-col">
      {header}
      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-4 py-3">{children}</div>
    </div>
  );
}

/**
 * 詳細の見出し。左に名前（`title`。等幅・太字）とその前後の印（`leading`・`trailing`）、下に補足（`subtitle`）、
 * 右に操作（`actions`。`DetailAction` など）と、`onClose` があれば閉じるボタン。`tone` を渡すと、系統図のカードと
 * 同じサブシステム色の淡い地と左の色の線にする。名前が ID でなく表示名なら `mono={false}`（ふつうの字形）
 */
export function DetailHeader({
  title,
  leading,
  trailing,
  subtitle,
  actions,
  tone,
  onClose,
  mono = true,
}: {
  title: string;
  leading?: ReactNode;
  trailing?: ReactNode;
  subtitle?: ReactNode;
  actions?: ReactNode;
  tone?: Tone;
  onClose?: () => void;
  /** 名前を等幅にするか（ID を出すとき。既定） */
  mono?: boolean;
}) {
  const t = useT().common;
  return (
    <div
      className={cn(
        "flex items-start justify-between gap-2 border-line border-b px-4 py-3",
        tone && ["border-l-4", toneBorder[tone]],
      )}
      style={tone ? { background: toneTint(tone, "header") } : undefined}
    >
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          {leading}
          <h2 className={cn("truncate font-semibold text-[15px]", mono && "font-mono")}>{title}</h2>
          {trailing}
        </div>
        {subtitle && <p className="mt-0.5 truncate text-ink-3 text-xs">{subtitle}</p>}
      </div>
      {(actions || onClose) && (
        <div className="flex shrink-0 gap-1">
          {actions}
          {onClose && (
            <Button
              size="icon-sm"
              variant="ghost"
              title={t.close}
              aria-label={t.close}
              onClick={onClose}
            >
              <X size={15} />
            </Button>
          )}
        </div>
      )}
    </div>
  );
}

/** 詳細の見出しの、アイコンだけの操作ボタン（名前の変更・削除など）。`label` は title と aria-label の両方に出す */
export function DetailAction({
  icon: Icon,
  label,
  onClick,
}: {
  icon: LucideIcon;
  label: string;
  onClick: () => void;
}) {
  return (
    <Button size="icon-sm" variant="outline" title={label} aria-label={label} onClick={onClick}>
      <Icon size={13} />
    </Button>
  );
}
