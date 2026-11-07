import { SkeletonPage, SkeletonPanel } from "@/kit/Loading";
import { LazyRoute } from "@/shell/LazyRoute";
import { loadOnNavigate } from "@/shell/lazy-page";
import { pages } from "@/shell/pages";

/** ルートを読んだ時点で本体も読み始める（描画を待たない） */
export const clientLoader = loadOnNavigate(pages.settings);

/** 本体のチャンクを読む間は、ページの骨組みのスケルトンを出す */
export default function Route() {
  return (
    <LazyRoute
      page={pages.settings}
      fallback={
        <SkeletonPage>
          <SkeletonPanel lines={3} />
          <SkeletonPanel lines={6} />
        </SkeletonPage>
      }
    />
  );
}
