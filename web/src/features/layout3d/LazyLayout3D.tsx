import { type ComponentProps, lazy, Suspense } from "react";
import { Loading } from "@/kit/Loading";
import { Skeleton } from "@/ui/skeleton";
import type { Layout3D } from "./Layout3D";

/** 3D 配置の本体（three.js を含む大きなチャンク）。構造・熱のナビにホバーしたときにも先に読む */
export const loadLayout3D = () => import("./Layout3D").then((m) => ({ default: m.Layout3D }));

const Layout3DBody = lazy(loadLayout3D);

/**
 * 3D 配置を遅れて読む。three.js を使うのは構造と熱だけなので、ほかの系のページはこのチャンクを読まない。
 * 読む間は同じ高さのスケルトンを出す（下のカードが跳ねないように）。
 */
export function LazyLayout3D(props: ComponentProps<typeof Layout3D>) {
  return (
    <Suspense
      fallback={
        <Loading>
          <Skeleton className="h-[502px] rounded-card" />
        </Loading>
      }
    >
      <Layout3DBody {...props} />
    </Suspense>
  );
}
