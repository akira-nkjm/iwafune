import { lazy, Suspense } from "react";
import { Loading, SkeletonBlock } from "@/kit/feedback/Loading";
import type { Layout3DProps } from "./Layout3D";

/** 3D 配置の本体（three.js を含む大きなチャンク）。構造・熱のナビにホバーしたときにも先に読む */
export const loadLayout3D = () => import("./Layout3D").then((m) => ({ default: m.Layout3D }));

const Layout3DBody = lazy(loadLayout3D);

/**
 * 3D 配置を遅れて読む。three.js を使うのは構造と熱だけなので、ほかの系のページはこのチャンクを読まない。
 * 読む間は同じ高さのスケルトンを出す（下のカードが跳ねないように）。用途は `variant`（`Layout3DProps`）。
 */
export function LazyLayout3D(props: Layout3DProps) {
  return (
    <Suspense
      fallback={
        <Loading>
          <SkeletonBlock className="h-[502px] rounded-card" />
        </Loading>
      }
    >
      <Layout3DBody {...props} />
    </Suspense>
  );
}
