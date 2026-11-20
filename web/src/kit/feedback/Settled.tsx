import { useIsFetching } from "@tanstack/react-query";
import { type ReactNode, useEffect, useState } from "react";
import { afterPaint } from "@/lib/after-paint";
import { cn } from "@/lib/cn";
import { FORMAT_KEY } from "@/quantity/format-batch";
import { Working } from "./Working";

/**
 * 中身がすべてそろうまで、ロゴの待ちの表示で待つ枠。そろってから一度に出すので、図が先に出て数字があとから
 * 埋まる、といったばらばらの現れ方をしない。待つのは 2 つ:
 * - `pending`: 呼び手のデータ（解析の結果など）がまだ無い間。この間は中身を描かない。
 * - 数量の表示文字列（`Formatted`。API が単位と桁を決める）の問い合わせ。中身を見えない状態で描いて問い合わせを
 *   始めさせ、すべて届いたら見せる。
 * 一度見せたあとは、読み直しがあっても隠さない（前の中身を出したままにするのは `Refreshable`）。
 * `className` は待ちの表示の置き場所（高さなど）。
 */
export function Settled({
  pending,
  label,
  className,
  children,
}: {
  pending: boolean;
  /** していること（翻訳済みの文） */
  label: string;
  className?: string;
  children: ReactNode;
}) {
  const formatting = useIsFetching({ queryKey: [FORMAT_KEY] });
  const [shown, setShown] = useState(false);
  useEffect(() => {
    if (shown || pending || formatting > 0) return;
    // 中身が問い合わせを始めるのは描いた直後なので、1 コマ待ってもまだ何も待っていなければ見せる
    return afterPaint(() => setShown(true));
  }, [shown, pending, formatting]);
  if (shown) return <>{children}</>;
  return (
    <div className="relative">
      <Working label={label} after={0} className={cn("min-h-64 py-10", className)} />
      {!pending && (
        <div aria-hidden="true" className="invisible absolute inset-0 overflow-hidden">
          {children}
        </div>
      )}
    </div>
  );
}
