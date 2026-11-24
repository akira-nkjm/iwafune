import { useQuery } from "@tanstack/react-query";
import { type ComponentType, lazy, type ReactNode, Suspense } from "react";
import { analysisOutput } from "@/api/analysis-output";
import { q } from "@/api/queries";
import type { AnalysisRunResult, ModalFemShapes, ThermalPlateTemperatures } from "@/api/types";
import { Loading, SkeletonBlock } from "@/kit/feedback/Loading";

/** 重い任意の出力（`AnalysisRunResult.detail`）の表示。型は API の型名（`AnalysisEntry.detail_type`）で、値は `unknown` で届く */
interface DetailView {
  /**
   * この結果に詳細があるか（結果の出力から決める）。偽なら、詳細を取りに行かず何も出さない（空の枠を出さない）。
   * 省略した型は、詳細を頼めば必ずある
   */
  available?: (result: AnalysisRunResult) => boolean;
  /** 表示の本体（three.js を含む大きなチャンク。行を開いて詳細が届いたときに初めて読む） */
  Body: ComponentType<{ result: AnalysisRunResult; detail: unknown }>;
}

const ModeShapesBody = lazy(() =>
  import("./ModeShapes").then((m) => ({
    default: ({ result, detail }: { result: AnalysisRunResult; detail: unknown }) => (
      <m.ModeShapes result={result} shapes={detail as ModalFemShapes} />
    ),
  })),
);

const PlateTemperaturesBody = lazy(() =>
  import("./PlateTemperatures").then((m) => ({
    default: ({ detail }: { result: AnalysisRunResult; detail: unknown }) => (
      <m.PlateTemperatures detail={detail as ThermalPlateTemperatures} />
    ),
  })),
);

/** 熱の解析の結果に、升に分けた板があるか（`couplings.plates`。定常と非定常で同じ詳細を持つ） */
function hasSubdividedPlates(result: AnalysisRunResult): boolean {
  const output =
    analysisOutput(result, "thermal.steady") ?? analysisOutput(result, "thermal.transient");
  return (output?.couplings.plates?.length ?? 0) > 0;
}

/**
 * **解析の重い任意の出力の表示の登録表（API の型名 → 表示）。** 解析の行を開いたときに、その解析が詳細を持てば（`detail_type`）
 * 別の問い合わせ（`detail: true`）で詳細を取り、ここで引いた表示で出す。型名の表に無い詳細は出さない。
 * メッシュ上の場を持つ次の解析は、ここに 1 行足す。
 */
const DETAIL_VIEWS: Record<string, DetailView> = {
  ModalFemShapes: { Body: ModeShapesBody },
  ThermalPlateTemperatures: { Body: PlateTemperaturesBody, available: hasSubdividedPlates },
};

/** この結果の、この型名の詳細を表示できるか（型名の表にあり、その結果に詳細がある） */
export function hasDetailView(detailType: string | undefined, result: AnalysisRunResult): boolean {
  if (detailType === undefined) return false;
  const view = DETAIL_VIEWS[detailType];
  return view !== undefined && (view.available?.(result) ?? true);
}

/**
 * 解析の行の詳細（開いた行の中）。詳細は、行の結果と同じケース・形態で別に取る（結果の入力が同じなので、同じ解析の続き）。
 * 取れるまでは骨組み、失敗は理由を出す。
 */
export function AnalysisDetail({
  result,
  detailType,
}: {
  result: AnalysisRunResult;
  detailType: string;
}): ReactNode {
  const view = hasDetailView(detailType, result) ? DETAIL_VIEWS[detailType] : undefined;
  const query = useQuery({
    ...q.analysisDetail(result.id, result.case, result.configuration ?? null),
    enabled: view !== undefined && result.status === "ok",
  });
  if (!view || result.status !== "ok") return null;
  if (query.error) return <p className="text-neg-ink text-sm">{query.error.message}</p>;
  const detail = query.data?.detail;
  if (detail === undefined || detail === null)
    return (
      <Loading>
        <SkeletonBlock className="h-[420px] rounded-control" />
      </Loading>
    );
  return (
    <Suspense
      fallback={
        <Loading>
          <SkeletonBlock className="h-[420px] rounded-control" />
        </Loading>
      }
    >
      <view.Body result={result} detail={detail} />
    </Suspense>
  );
}
