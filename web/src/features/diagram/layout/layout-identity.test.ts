import { beforeAll, describe, expect, it } from "vitest";
import { layoutCases } from "@/test/layout-cases";
import golden from "@/test/layout-golden.json";
import { sha256, snapshotOf } from "@/test/layout-snapshot";
import { LAYOUT_RESULT_DIGESTS } from "@/test/layout-versions";
import { layoutModel } from "./layout";
import { LAYOUT_VERSION } from "./plan-key";

/** 全部の入力を配置するのに掛かる時間の上限（CI の runner は手元より数倍遅いことがある） */
const TIMEOUT_MS = 120_000;

describe("配置の結果は変わらない", () => {
  const out: Record<string, unknown> = {};
  beforeAll(async () => {
    for (const c of layoutCases) out[c.name] = snapshotOf(await layoutModel(c.model, c.full));
  }, TIMEOUT_MS);

  // layout-golden.json は、速くする前の配置（探索の書き換え・端子の選び直しでの引き直しの省略の前）の結果。
  // 位置・経路・ラベル・端子・分かれ口が 1 つも違わないこと
  for (const c of layoutCases)
    it(`${c.name}: 位置・経路・ラベルが写しと同じ`, () => {
      expect(out[c.name]).toEqual((golden as Record<string, unknown>)[c.name]);
    });

  it("結果が変わったら LAYOUT_VERSION を上げ、版の行を足す（残した結果を古いまま使わないため）", async () => {
    const digest = await sha256(JSON.stringify(out));
    expect(
      LAYOUT_RESULT_DIGESTS[LAYOUT_VERSION],
      `配置の結果が版 ${LAYOUT_VERSION} の写しと違う（${digest}）。アルゴリズムを変えたなら LAYOUT_VERSION を上げて test/layout-versions.ts に行を足す`,
    ).toBe(digest);
    expect(Math.max(...Object.keys(LAYOUT_RESULT_DIGESTS).map(Number))).toBe(LAYOUT_VERSION);
  });
});
