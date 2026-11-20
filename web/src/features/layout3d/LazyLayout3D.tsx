import { lazy, Suspense } from "react";
import { useT } from "@/i18n";
import { Loading, SkeletonBlock } from "@/kit/feedback/Loading";
import { Working } from "@/kit/feedback/Working";
import type { Layout3DProps } from "./Layout3D";

/** 3D 配置の本体（three.js を含む大きなチャンク）。配置・熱のナビにホバーしたときにも先に読む */
export const loadLayout3D = () => import("./Layout3D").then((m) => ({ default: m.Layout3D }));

const Layout3DBody = lazy(loadLayout3D);

/**
 * 3D 配置を遅れて読む。three.js を使うのは配置と熱だけなので、ほかの系のページはこのチャンクを読まない。
 * 読む間は同じ高さのスケルトンを出す（下のカードが跳ねないように）。用途は `variant`（`Layout3DProps`）。
 */
export function LazyLayout3D(props: Layout3DProps) {
  const t = useT();
  const fill = props.variant === "placement";
  return (
    <Suspense
      fallback={
        // チャンクの読み込みが 1 秒を超えるときだけ、スケルトンの上に中央のロゴとして出す
        <div className={fill ? "relative flex min-h-0 flex-1 flex-col" : "relative"}>
          <Loading className={fill ? "flex min-h-0 flex-1 flex-col" : undefined}>
            <SkeletonBlock
              className={fill ? "min-h-0 flex-1 rounded-card" : "h-[502px] rounded-card"}
            />
          </Loading>
          <Working
            label={t.common.working.drawing3d}
            announce={false}
            className="absolute inset-0"
          />
        </div>
      }
    >
      <Layout3DBody {...props} />
    </Suspense>
  );
}
