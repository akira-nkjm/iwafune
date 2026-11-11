import type { ReactNode } from "react";
import type { DesignView } from "@/api/types";
import { type QueryLike, QueryState } from "../feedback/QueryState";
import { DetailLayout, DetailSkeleton } from "./DetailLayout";
import type { DetailRoute } from "./use-detail-route";

/**
 * 一覧と詳細のページの骨組み（部品・カタログ）。見出し（`header`。検索欄・追加ボタンはページが `PageHeader` に
 * 差し込む）とタブ（`tabs`）の下に、設計の読み込み中・失敗・成功を `QueryState` で出し分ける。読み込み中は選んだ表示方法の
 * 配置のスケルトン、読み込んだら一覧と詳細を `DetailLayout` で並べる（表は全幅とモーダル、ツリーと `wideList` は横の欄）。
 *
 * `list` は読み込んだ設計から一覧を、`detail` は URL で選んだ ID の詳細を作る（設計に無い ID なら null を返す）。
 * 選択・表示方法・追加ダイアログの開閉は `route`（`useDetailRoute`）。追加ダイアログはページが自分で描く。
 */
export function ListDetailPage({
  header,
  tabs,
  design,
  route,
  wideList = false,
  hint,
  list,
  detail,
}: {
  header: ReactNode;
  tabs?: ReactNode;
  /** 設計の問い合わせ（`useDesign()` の戻り値） */
  design: QueryLike<DesignView>;
  route: DetailRoute;
  wideList?: boolean;
  /** 何も選んでいないときに詳細の欄に出す文言 */
  hint: string;
  list: (design: DesignView) => ReactNode;
  detail: (design: DesignView, id: string) => ReactNode;
}) {
  return (
    <>
      {header}
      {tabs}
      <QueryState
        query={design}
        className="flex min-h-0 flex-1 flex-col"
        skeleton={<DetailSkeleton tree={route.tree} />}
      >
        {(d) => (
          <DetailLayout
            tree={route.tree}
            wideList={wideList}
            title={route.selected ?? ""}
            hint={hint}
            onClose={() => route.select(null)}
            list={list(d)}
            detail={route.selected ? detail(d, route.selected) : null}
          />
        )}
      </QueryState>
    </>
  );
}
