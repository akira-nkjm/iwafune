import { handlePlanRequest, type PlanReply, type PlanRequest } from "./plan-worker-core";

/**
 * 配置を計算するワーカー（主スレッドを止めないために、`planLayout` をここで回す）。
 * 依頼を受けて、返事（結果か失敗）を返すだけ。
 */
interface WorkerScope {
  onmessage: ((e: { data: PlanRequest }) => void) | null;
  postMessage(reply: PlanReply): void;
}
const scope = self as unknown as WorkerScope;
scope.onmessage = (e) => scope.postMessage(handlePlanRequest(e.data));
