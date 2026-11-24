import { MutationObserver, QueryClient, QueryObserver } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiError } from "./client";
import { keys } from "./keys";
import { applyOpsOptions, dryRunOptions } from "./ops";
import type { Op } from "./types";

// 模擬のサーバー: 確定すると revision が進み、消す Op の対象は無くなり、足す Op の対象は既に在る
const server = { revision: 1, exists: true, dryRuns: [] as { revision: string | null }[] };

vi.mock("./client", async (original) => ({
  ...(await original<typeof import("./client")>()),
  api: vi.fn(async (path: string, init?: { body?: unknown }) => {
    if (path === "/design") {
      // 読み直しには時間がかかる（その間、ダイアログは開いたまま）
      if (server.revision > 1) await new Promise((resolve) => setTimeout(resolve, 20));
      return { project: { revision: `r${server.revision}` }, defs: { budget_kinds: [] } };
    }
    if (path === "/diagnostics") {
      if (server.revision > 1) await new Promise((resolve) => setTimeout(resolve, 60));
      return [];
    }
    const body = init?.body as { dry_run: boolean; base_revision: string | null };
    if (body.dry_run) {
      server.dryRuns.push({ revision: body.base_revision });
      // 確定のあとは、同じ Op を計画できない（消す Op は対象が無く、足す Op は既に在る）
      if (!server.exists) throw new ApiError(404, "not_found", "not found");
      return { cli: ["fune x"], changed: [], diagnostics: [] };
    }
    server.revision += 1;
    server.exists = false;
    return { cli: ["fune x"], changed: [], diagnostics: [] };
  }),
}));

const OPS: Op[] = [{ op: "remove_maneuver", id: "RAISE" }];
const settle = () => new Promise((resolve) => setTimeout(resolve, 50));

describe("確定のあとの dry_run", () => {
  let qc: QueryClient;
  beforeEach(() => {
    server.revision = 1;
    server.exists = true;
    server.dryRuns = [];
    qc = new QueryClient();
  });
  afterEach(() => qc.clear());

  it("確定が成功して revision が進んでも、同じ Op の dry_run を取り直さない", async () => {
    // 設計は画面が見ている（確定の無効化で読み直され、revision が進む）
    const design = new QueryObserver(qc, {
      queryKey: keys.design,
      queryFn: () => import("./client").then((m) => m.api("/design")),
    });
    const stopDesign = design.subscribe(() => {});
    // 診断も画面が見ている（無効化の完了は、遅いほうを待つ。設計はもう新しい revision になっている）
    const diagnostics = new QueryObserver(qc, {
      queryKey: keys.diagnostics,
      queryFn: () => import("./client").then((m) => m.api("/diagnostics")),
    });
    const stopDiagnostics = diagnostics.subscribe(() => {});
    await settle();
    // ダイアログの `useDryRun` と同じ: 確定の最中かを見て、問い合わせの設定を作り直す
    const applyingNow = () => qc.isMutating({ mutationKey: keys.applyOps }) > 0;
    const dry = new QueryObserver(qc, dryRunOptions(qc, OPS, applyingNow()));
    let last = "";
    const rebuild = () => {
      // hook は描き直しのときだけ設定を作り直す。キーか確定の最中かが変わったときだけ作り直す
      const next = dryRunOptions(qc, OPS, applyingNow());
      const sig = JSON.stringify([next.queryKey, next.enabled]);
      if (sig === last) return;
      last = sig;
      dry.setOptions(next);
    };
    const stopDry = dry.subscribe(() => {});
    // React は状態の変化を同じ tick では描き直さない（確定の `.then` で閉じるほうが先）ので、遅らせて作り直す
    let timer: ReturnType<typeof setTimeout> | undefined;
    const later = () => {
      clearTimeout(timer);
      timer = setTimeout(rebuild, 0);
    };
    const stopCache = qc.getQueryCache().subscribe(later);
    const stopMutations = qc.getMutationCache().subscribe(later);
    await settle();
    expect(server.dryRuns).toHaveLength(1);

    await new MutationObserver(qc, applyOpsOptions(qc)).mutate({ ops: OPS, summary: "x" });
    // ダイアログは確定の成功で閉じる（`OpDialog` の `.then`）。閉じたら問い合わせの設定は作り直されない
    for (const stop of [stopDry, stopCache, stopMutations]) stop();
    clearTimeout(timer);
    await settle();

    expect(qc.getQueryData<{ project: { revision: string } }>(keys.design)?.project.revision).toBe(
      "r2",
    );
    // 確定の前に 1 回だけ。確定のあとの取り直し（404 になる）は無い
    expect(server.dryRuns).toHaveLength(1);
    stopDesign();
    stopDiagnostics();
  });

  it("確定が弾かれたら、dry_run はそのまま使える（止まったままにならない）", async () => {
    const applyingNow = () => qc.isMutating({ mutationKey: keys.applyOps }) > 0;
    const dry = new QueryObserver(qc, dryRunOptions(qc, OPS, applyingNow()));
    const stopDry = dry.subscribe(() => {});
    await settle();
    expect(dry.getCurrentResult().isSuccess).toBe(true);
    // 確定の最中は新しい問い合わせを始めない
    dry.setOptions(dryRunOptions(qc, [{ op: "remove_maneuver", id: "OTHER" }], true));
    await settle();
    expect(server.dryRuns).toHaveLength(1);
    // 確定が終われば、また問い合わせる
    dry.setOptions(dryRunOptions(qc, [{ op: "remove_maneuver", id: "OTHER" }], false));
    await settle();
    expect(server.dryRuns).toHaveLength(2);
    stopDry();
  });
});
