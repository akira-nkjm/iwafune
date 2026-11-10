import { SkeletonPage, SkeletonTable } from "@/kit/Loading";
import { LazyRoute } from "@/shell/LazyRoute";
import { loadOnNavigate } from "@/shell/lazy-page";
import { pages } from "./pages";

/** ルートを読んだ時点で本体も読み始める（描画を待たない） */
export const clientLoader = loadOnNavigate(pages.modes);

/** 本体のチャンクを読む間は、ページの骨組みのスケルトンを出す */
export default function Route() {
  return (
    <LazyRoute
      page={pages.modes}
      fallback={
        <SkeletonPage body={false}>
          <SkeletonTable rows={10} cols={6} className="border-line border-t" />
        </SkeletonPage>
      }
    />
  );
}
