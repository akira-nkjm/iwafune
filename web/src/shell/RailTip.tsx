import type { ReactNode } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/ui/tooltip";

/**
 * レール（畳んだサイドバー）のときだけ、名前をツールチップで出す。開いているときは名前が見えているので包まない。
 * 子は className を文字列で持つこと（asChild は className を文字列として結合する）。
 */
export function RailTip({
  rail,
  label,
  children,
}: {
  rail: boolean;
  label: ReactNode;
  children: ReactNode;
}) {
  if (!rail) return children;
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  );
}
