import { SkeletonCards, SkeletonPage, SkeletonPanel } from "@/kit/Loading";
import { LazyRoute } from "@/shell/LazyRoute";
import { loadOnNavigate } from "@/shell/lazy-page";
import { pages } from "./pages";

/** ルートを読んだ時点で本体も読み始める（描画を待たない） */
export const clientLoader = loadOnNavigate(pages.overview);

/** 本体のチャンクを読む間は、ページの骨組みのスケルトンを出す */
export default function Route() {
  return <LazyRoute page={pages.overview} fallback={<OverviewSkeleton />} />;
}

function OverviewSkeleton() {
  return (
    <SkeletonPage>
      <SkeletonCards count={7} />
      <div className="grid gap-4 xl:grid-cols-2">
        <SkeletonPanel lines={5} />
        <SkeletonPanel lines={5} />
      </div>
    </SkeletonPage>
  );
}
