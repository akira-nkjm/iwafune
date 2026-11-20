import { useParams } from "react-router";
import { SkeletonPage, SkeletonPanel } from "@/kit/feedback/Loading";
import { LazyRoute } from "@/shell/LazyRoute";
import { loadOnNavigate } from "@/shell/lazy-page";
import { pages } from "./pages";

/** ルートを読んだ時点で本体も読み始める（描画を待たない） */
export const clientLoader = loadOnNavigate(pages.system);

/**
 * 本体を描くまでは、ページの骨組みのスケルトンを出す。系を替えたら別のページとして描き直す（`key`）。
 * タブの切り替えは同じページのまま。
 */
export default function Route() {
  const { system } = useParams();
  return <LazyRoute key={system} page={pages.system} fallback={<SystemSkeleton />} />;
}

function SystemSkeleton() {
  return (
    <SkeletonPage kpis tabs>
      <div className="grid gap-4 xl:grid-cols-2">
        <SkeletonPanel lines={5} />
        <SkeletonPanel lines={5} />
      </div>
    </SkeletonPage>
  );
}
