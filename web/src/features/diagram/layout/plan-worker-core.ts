import type { DiagramModel } from "../model";
import { type LayoutPlan, planLayout } from "./layout";

/** 配置の計算の依頼（`id` は返事と突き合わせる番号） */
export interface PlanRequest {
  id: number;
  full: DiagramModel;
}

/** 配置の計算の返事（成功か、失敗の理由） */
export type PlanReply = { id: number; plan: LayoutPlan } | { id: number; error: string };

/** 依頼を 1 つ処理する（ワーカーの中でも、ワーカーを使えないときの主スレッドでも同じ） */
export function handlePlanRequest(req: PlanRequest): PlanReply {
  try {
    return { id: req.id, plan: planLayout(req.full) };
  } catch (e) {
    return { id: req.id, error: e instanceof Error ? e.message : String(e) };
  }
}
