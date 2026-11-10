import { SkeletonPage, SkeletonPanel } from "@/kit/Loading";
import { LazyRoute } from "@/shell/LazyRoute";
import { loadOnNavigate } from "@/shell/lazy-page";
import { pages } from "./pages";

/** ルートを読んだ時点で本体も読み始める（描画を待たない） */
export const clientLoader = loadOnNavigate(pages.orbit);

/** 本体のチャンクを読む間は、ページの骨組みのスケルトンを出す */
export default function Route() {
  return (
    <LazyRoute
      page={pages.orbit}
      fallback={
        <SkeletonPage>
          <SkeletonPanel lines={6} />
          <SkeletonPanel lines={4} />
        </SkeletonPage>
      }
    />
  );
}
