import type { AnalysisInfo } from "@/api/types";
import { localized } from "./index";

/**
 * 解析の名前と説明。正本は sat-analysis（API の title / title_en、description / description_en）。
 * 英語の表示では title_en を使い、空なら日本語のまま出す（ここに英語を重ねて持たない）。
 */
export function analysisTitle(
  info: Pick<AnalysisInfo, "title" | "title_en"> | undefined,
  fallback: string,
): string {
  return info ? localized(info.title, info.title_en) : fallback;
}

export function analysisDescription(
  info: Pick<AnalysisInfo, "description" | "description_en">,
): string {
  return localized(info.description, info.description_en);
}
