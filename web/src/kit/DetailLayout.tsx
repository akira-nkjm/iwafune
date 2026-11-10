import type { ReactNode } from "react";
import { DetailDialog } from "./DetailDialog";
import { Empty } from "./Empty";
import { SkeletonLines, SkeletonTable } from "./Loading";
import { TreePane } from "./TreePane";
import { readTreeWidth } from "./tree-width";

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

/**
 * 読み込み中の骨組み。選んだ表示方法の配置（表は全幅、ツリーは一覧と詳細の 2 列。列幅は `TreePane` と同じ保存値）で出す。
 * 動かせる境目は置かない（読み込み中の印の中に、フォーカスできる要素を入れない）。`kit/Loading` で包むのは呼び手。
 */
export function DetailSkeleton({ tree }: { tree: boolean }) {
  if (!tree) return <SkeletonTable className="min-h-0 flex-1" />;
  return (
    <div
      className="grid min-h-0 flex-1 border-line border-t"
      style={{ gridTemplateColumns: `${readTreeWidth()}px 6px minmax(0, 1fr)` }}
    >
      <SkeletonTable cols={2} />
      <div className="border-line border-x bg-panel-2" />
      <div className="p-4">
        <SkeletonLines lines={8} />
      </div>
    </div>
  );
}
