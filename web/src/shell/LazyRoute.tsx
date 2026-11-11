import { type ComponentType, type ReactNode, useEffect, useState } from "react";
import type { LazyPage } from "./lazy-page";

/**
 * ルートの中身。ページの本体を読み終わっていればそのまま描き、まだなら骨組み（fallback）を出して、読み終わったら差し替える。
 * Suspense にしないのは、React が骨組みを出してから 300 ms は中身の表示を待たせる（FALLBACK_THROTTLE）ため。
 * 骨組みは kit/feedback/Loading の「少し待ってから現れる」演出を持つので、速く読めたときはちらつかない。
 * 読めなかったときは描画中に投げて、ルートのエラー表示（root.tsx の ErrorBoundary）に任せる。
 */
export function LazyRoute({ page, fallback }: { page: LazyPage; fallback: ReactNode }) {
  const [Page, setPage] = useState<ComponentType | null>(() => page.current);
  const [error, setError] = useState<unknown>(null);
  useEffect(() => {
    if (Page) return;
    let live = true;
    page.load().then(
      (c) => {
        if (live) setPage(() => c);
      },
      (e: unknown) => {
        if (live) setError(e ?? new Error("load failed"));
      },
    );
    return () => {
      live = false;
    };
  }, [page, Page]);
  if (error) throw error;
  return Page ? <Page /> : fallback;
}
