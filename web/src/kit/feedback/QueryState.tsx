import { RotateCw } from "lucide-react";
import type { ReactNode } from "react";
import { useT } from "@/i18n";
import { Button } from "@/ui/button";
import { Empty } from "./Empty";
import { Loading } from "./Loading";

/**
 * 問い合わせの結果のうち、`QueryState` が見るところ。TanStack Query の `useQuery` の戻り値をそのまま渡せる。
 * 値から作った別のもの（設計から作った図など）を待つときは、`{ data, error, refetch }` を組んで渡す
 * （戻り値を `...` で広げない。TanStack Query は読んだ欄だけを見張るので、広げると状態が変わるたびに描き直す）。
 */
export type QueryLike<T> = {
  data: T | undefined;
  error: { message: string } | null;
  refetch: () => unknown;
};

/** 問い合わせの今の段階（値があれば、裏で読み直しに失敗していても値を出す） */
export type QueryPhase<T> =
  | { phase: "ready"; data: T }
  | { phase: "failed"; message: string }
  | { phase: "loading" };

/** 問い合わせの結果 → 今の段階（`QueryState` の分岐。DOM を使わずに確かめられるよう分けてある） */
export function queryPhase<T>(query: Pick<QueryLike<T>, "data" | "error">): QueryPhase<T> {
  if (query.data !== undefined) return { phase: "ready", data: query.data };
  if (query.error) return { phase: "failed", message: query.error.message };
  return { phase: "loading" };
}

/**
 * 問い合わせの読み込み中・失敗・成功の出し分け。読み込み中は `skeleton`（`kit/feedback/Loading` の Skeleton* で形を作る）を
 * `Loading` で包んで出し、失敗は理由（`failure`。既定は設計を読めない旨）と再試行のボタン、読めたら `children` に値を渡す。
 * `className` は読み込み中の枠の class（並べ方・余白）。
 */
export function QueryState<T>({
  query,
  skeleton,
  failure,
  className,
  children,
}: {
  query: QueryLike<T>;
  skeleton: ReactNode;
  failure?: (message: string) => string;
  className?: string;
  children: (data: T) => ReactNode;
}) {
  const state = queryPhase(query);
  if (state.phase === "ready") return children(state.data);
  if (state.phase === "failed")
    return <QueryFailure message={state.message} failure={failure} onRetry={query.refetch} />;
  return <Loading className={className}>{skeleton}</Loading>;
}

/** 読めなかったときの理由と再試行のボタン */
function QueryFailure({
  message,
  failure,
  onRetry,
}: {
  message: string;
  failure: ((message: string) => string) | undefined;
  onRetry: () => unknown;
}) {
  const t = useT().common;
  return (
    <Empty>
      <p role="alert">{(failure ?? t.cannotLoadDesign)(message)}</p>
      <Button variant="outline" size="sm" className="mt-3" onClick={() => void onRetry()}>
        <RotateCw size={13} />
        {t.retry}
      </Button>
    </Empty>
  );
}
