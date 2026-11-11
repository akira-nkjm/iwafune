import { X } from "lucide-react";
import { type ReactNode, useRef } from "react";
import { useT } from "@/i18n";
import { Button } from "@/ui/button";
import { Dialog, DialogClose, DialogContent, DialogTitle } from "@/ui/dialog";

/** URL 選択で開く詳細。Radix のフォーカストラップと、元の表の行への復帰。 */
export function DetailDialog({
  detail,
  title,
  onClose,
}: {
  detail: ReactNode;
  title: string;
  onClose: () => void;
}) {
  const t = useT().common;
  const opener = useRef<HTMLElement | null>(null);
  return (
    <Dialog
      open={!!detail}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <DialogContent
        showCloseButton={false}
        aria-describedby={undefined}
        className="flex h-[85dvh] max-h-[900px] flex-col gap-0 overflow-hidden p-0 sm:max-w-3xl"
        onOpenAutoFocus={() => {
          opener.current =
            document.activeElement instanceof HTMLElement ? document.activeElement : null;
        }}
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          if (opener.current?.isConnected) opener.current.focus();
        }}
      >
        <div className="flex shrink-0 items-center justify-between border-line border-b px-4 py-2">
          <DialogTitle className="truncate font-mono text-sm">{title}</DialogTitle>
          <DialogClose asChild>
            <Button variant="ghost" size="icon-sm" aria-label={t.close}>
              <X size={16} />
            </Button>
          </DialogClose>
        </div>
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden [&>div]:flex-1">{detail}</div>
      </DialogContent>
    </Dialog>
  );
}
