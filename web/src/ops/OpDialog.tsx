import type { ReactNode } from "react";
import { useApplyOps } from "@/api/ops";
import type { Op } from "@/api/types";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { Button } from "@/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/ui/dialog";
import { OpPreview } from "./OpPreview";
import { useOpCheck } from "./use-op-check";

/**
 * 編集のダイアログの共通形: フォーム + 同じ操作の CLI（dry_run）+ 確定。
 * ops が null の間は確定できない（入力が足りない・不正）。確定すると Op を送り、成功したら閉じる。
 */
export function OpDialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  ops,
  summary,
  confirmLabel,
  destructive,
  wide,
  onBeforeApply,
  onDone,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: ReactNode;
  children?: ReactNode;
  ops: Op[] | null;
  summary: string;
  confirmLabel?: string;
  destructive?: boolean;
  wide?: boolean;
  /** 送る直前（設計の再読込の前）に呼ぶ。消すものを見ている画面が、消えたものを読みに行かないよう移るのに使う */
  onBeforeApply?: () => void;
  onDone?: () => void;
}) {
  const t = useT();
  const apply = useApplyOps();
  // 確定できるのは dry_run が通ったときだけ（規則はサーバーが持つ）
  const check = useOpCheck(open ? ops : null);
  const submit = () => {
    if (!ops) return;
    onBeforeApply?.();
    // 設計の再読込で詳細が消えても、閉じる処理と選び直しを最後まで実行する。
    void apply.mutateAsync({ ops, summary }).then(
      () => {
        onOpenChange(false);
        onDone?.();
      },
      // エラー表示は mutation の状態に任せ、ダイアログは開いたままにする。
      () => {},
    );
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={cn(wide ? "sm:max-w-2xl" : "sm:max-w-md")}>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description && <DialogDescription>{description}</DialogDescription>}
        </DialogHeader>
        <form
          className="grid min-w-0 gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          {children}
          {open && <OpPreview ops={ops} />}
          {apply.isError && <p className="text-neg-ink text-xs">{apply.error.message}</p>}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              {t.common.cancel}
            </Button>
            <Button
              type="submit"
              variant={destructive ? "destructive" : "default"}
              disabled={!check.ok || apply.isPending}
            >
              {apply.isPending ? t.common.applying : (confirmLabel ?? t.common.apply)}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
