import { SkeletonBlock, SkeletonPage } from "@/kit/feedback/Loading";
import { LazyRoute } from "@/shell/LazyRoute";
import { loadOnNavigate } from "@/shell/lazy-page";
import { pages } from "./pages";

export const clientLoader = loadOnNavigate(pages.layout);

/** 配置ページの本体を読む間も残りの画面高を使う。 */
export default function Route() {
  return (
    <LazyRoute
      page={pages.layout}
      fallback={
        <SkeletonPage body={false}>
          <SkeletonBlock className="mx-6 mb-4 min-h-0 flex-1 rounded-card" />
        </SkeletonPage>
      }
    />
  );
}
