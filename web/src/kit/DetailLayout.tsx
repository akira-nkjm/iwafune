import type { ReactNode } from "react";
import { DetailDialog } from "./DetailDialog";
import { Empty } from "./Empty";
import { Loading, SkeletonLines, SkeletonTable } from "./Loading";
import { TreePane } from "./TreePane";

/**
 * 表は全幅とモーダル、ツリーは可変幅の一覧と残り幅の詳細。`wideList`（カードの一覧）は一覧が残りの幅を使い、
 * 詳細は選んだときだけ右に可変幅の欄で出す。
 */
export function DetailLayout({
  tree,
  wideList = false,
  list,
  detail,
  title,
  hint,
  onClose,
}: {
  tree: boolean;
  wideList?: boolean;
  list: ReactNode;
  detail: ReactNode;
  title: string;
  hint: string;
  onClose: () => void;
}) {
  if (wideList) return <TreePane list={list} detail={detail} fixed="detail" />;
  if (tree)
    return <TreePane list={list} detail={detail ?? <Empty className="m-auto">{hint}</Empty>} />;
  return (
    <>
      <div className="min-h-0 flex-1 overflow-hidden border-line border-t">{list}</div>
      <DetailDialog detail={detail} title={title} onClose={onClose} />
    </>
  );
}

/** 読み込み中も、選んだ表示方法の配置で既存スケルトンを出す。 */
export function DetailLoading({ tree }: { tree: boolean }) {
  if (!tree)
    return (
      <Loading className="min-h-0 flex-1">
        <SkeletonTable />
      </Loading>
    );
  return (
    <TreePane
      list={
        <Loading>
          <SkeletonTable cols={2} />
        </Loading>
      }
      detail={
        <Loading className="p-4">
          <SkeletonLines lines={8} />
        </Loading>
      }
    />
  );
}
