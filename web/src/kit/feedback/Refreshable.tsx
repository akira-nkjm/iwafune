import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { Refreshing } from "./Loading";

/**
 * 読み直し・再計算の間も前の中身を出しておく枠。`active` の間は、中身を少し薄くし、上端に細い帯を出す
 * （`Refreshing`。少したつとロゴと `label`・経過秒のチップも出る）。中身は動かさないので、枠の大きさは変わらない。
 * 薄くする（`opacity-50` など）書き方は個々の画面に書かず、この枠を使う。
 * `className` は枠の class（高さの伸ばし方など。`relative` はこの枠が付ける）。
 * `blockInput` のときは、読み直す間は中身を操作できなくする（図のように、古い位置を触らせたくないとき）。
 */
export function Refreshable({
  active,
  label,
  blockInput = false,
  className,
  children,
}: {
  active: boolean;
  /** していることを短く（翻訳済みの文。例: 「電源を再計算中」） */
  label: string;
  blockInput?: boolean;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={cn("relative", className)}>
      {active && <Refreshing label={label} />}
      <div
        aria-busy={active}
        className={cn(
          "flex h-full min-h-0 flex-1 flex-col transition-opacity duration-200 motion-reduce:transition-none",
          active && "opacity-55",
          active && blockInput && "pointer-events-none",
        )}
      >
        {children}
      </div>
    </div>
  );
}
