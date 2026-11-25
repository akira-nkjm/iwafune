import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { Refreshing } from "./Loading";
import { REFRESH_MARK_DELAY_MS } from "./use-working";
import { Working } from "./Working";

/**
 * 読み直しの間、前の中身を薄くして出しておくときの class（`active` のときだけ付く）。薄くする度合いと遷移はここだけが持つ。
 * 枠にできない要素（表の行など）はこれを直接使い、枠にできるなら `Refreshable` を使う。
 */
export function refreshingDim(active: boolean): string {
  return active ? "opacity-55 transition-opacity duration-200 motion-reduce:transition-none" : "";
}

/**
 * 読み直し・再計算の間も前の中身を出しておく枠。`active` の間は、中身を少し薄くし、上端に細い帯を出す
 * （`Refreshing`）。`label` を渡した広い場所では、少したつと中央にロゴのマーク・`label`・経過秒も出る（`Working`）。中身は動かさないので、枠の大きさは変わらない。
 * 薄くする（`opacity-50` など）書き方は個々の画面に書かず、この枠（枠にできない要素は `refreshingDim`）を使う。
 * `className` は枠の class（高さの伸ばし方など。`relative` はこの枠が付ける）。
 * `blockInput` のときは、読み直す間は中身を操作できなくする（図のように、古い位置を触らせたくないとき）。
 */
export function Refreshable({
  active,
  label,
  blockInput = false,
  immediate = false,
  className,
  children,
}: {
  active: boolean;
  /**
   * していることを短く（翻訳済みの文）。渡すと、中央にロゴのマークと経過秒も出す。図のような広い場所だけに使い、
   * 表やカードのような小さい場所では渡さない（薄くするのと上端の帯だけで足りる）。
   */
  label?: string;
  blockInput?: boolean;
  /**
   * 中央のロゴを待たずに最初から出す。読み直しが画面を止める計算（図の位置の計算）のときに使う。
   * 計算の間は画面を描き直せないので、始まる前に出しておかないとロゴが出ない。
   */
  immediate?: boolean;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={cn("relative", className)}>
      {active && <Refreshing />}
      {active && label && (
        // 薄くした中身の字と重なっても読めるように、地のある小さな札に入れて中央に置く
        <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
          <Working
            label={label}
            after={immediate ? 0 : REFRESH_MARK_DELAY_MS}
            announce={false}
            className="rounded-card border border-line bg-panel px-5 py-4 shadow-sm"
          />
        </div>
      )}
      <div
        aria-busy={active}
        className={cn(
          "flex h-full min-h-0 flex-1 flex-col transition-opacity duration-200 motion-reduce:transition-none",
          refreshingDim(active),
          active && blockInput && "pointer-events-none",
        )}
      >
        {children}
      </div>
    </div>
  );
}
