import { type ComponentType, type ReactNode, startTransition, useEffect, useState } from "react";
import { afterPaint } from "@/lib/after-paint";
import type { LazyPage } from "./lazy-page";

/**
 * ルートの中身。ページを移ったら、まず骨組み（fallback）だけを描いて画面に出し、そのあとでページの本体を描く。
 * 本体の最初の描画が重いページ（図・3D・大きな表）でも、押した直後に画面が切り替わる（前のページのまま固まらない）。
 * 本体は優先度の低い更新（transition）で描くので、描いている間も骨組みの動きは止まらない。
 * 本体のチャンクをまだ読んでいなければ、読み終わるまで骨組みのまま。
 * Suspense にしないのは、React が骨組みを出してから 300 ms は中身の表示を待たせる（FALLBACK_THROTTLE）ため。
 * 骨組みは kit/feedback/Loading の「少し待ってから現れる」演出を持つので、速く描けたときはちらつかない。
 * 読めなかったときは描画中に投げて、ルートのエラー表示（root.tsx の ErrorBoundary）に任せる。
 * 同じルートの中でページを替えるとき（系のページの系）は、呼び手が `key` を替えて、同じ手順をやり直す。
 */
export function LazyRoute({ page, fallback }: { page: LazyPage; fallback: ReactNode }) {
  const [Page, setPage] = useState<ComponentType | null>(null);
  const [error, setError] = useState<unknown>(null);
  useEffect(() => {
    let live = true;
    const show = (c: ComponentType) => {
      if (live) startTransition(() => setPage(() => c));
    };
    const fail = (e: unknown) => {
      if (live) setError(e ?? new Error("load failed"));
    };
    // 骨組みが画面に出てから本体を描く（読み終わっていれば、すぐ描き始める）
    const cancel = afterPaint(() => {
      if (page.current) show(page.current);
      else page.load().then(show, fail);
    });
    return () => {
      live = false;
      cancel();
    };
  }, [page]);
  if (error) throw error;
  return Page ? <Page /> : fallback;
}
