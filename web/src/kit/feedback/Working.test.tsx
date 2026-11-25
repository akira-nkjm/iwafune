import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { MARK } from "@/shell/brand/brand-art";
import { Refreshable, refreshingDim } from "./Refreshable";
import { Settled } from "./Settled";
import { Working } from "./Working";

describe("Working（長い待ちの中央表示）", () => {
  const out = renderToStaticMarkup(<Working label="システム図の配置を計算中" after={0} />);

  it("ロゴのマークの 3 つの面を brand-art の図形から描き、面ごとに時間をずらす", () => {
    for (const face of MARK.faces) expect(out).toContain(`d="${face.d}"`);
    expect(out.match(/mark-glow/g)).toHaveLength(MARK.faces.length);
    // 黄が差すのは甲板の面 1 つだけ
    expect(out.match(/mark-glow-gold/g)).toHaveLength(1);
    expect(out).toContain("--mark-i:2");
  });

  it("画面読み上げには状態と文だけを伝え、動く秒数は隠す", () => {
    expect(out).toContain('role="status"');
    expect(out).toContain('aria-busy="true"');
    expect(out).toContain("sr-only");
    expect(out).toMatch(/aria-hidden="true"[^>]*>0 秒/);
  });

  it("速い読み込みでは、待ち時間が過ぎるまで何も出さない", () => {
    expect(renderToStaticMarkup(<Working label="x" />)).toBe("");
  });

  it("Loading の中で使うとき（announce なし）は読み上げの領域を重ねない", () => {
    const quiet = renderToStaticMarkup(<Working label="x" after={0} announce={false} />);
    expect(quiet).not.toContain('role="status"');
  });
});

describe("Refreshable（前の中身を残したまま読み直す枠）", () => {
  const html = (active: boolean, blockInput = false) =>
    renderToStaticMarkup(
      <Refreshable active={active} label="電源を再計算中" blockInput={blockInput}>
        <p>前の結果</p>
      </Refreshable>,
    );

  it("読み直していなければ、中身をそのまま出し、帯も薄めもない", () => {
    const out = html(false);
    expect(out).toContain("前の結果");
    expect(out).not.toContain("refresh-bar");
    expect(out).not.toContain("opacity-55");
  });

  it("読み直している間は、前の中身を残して薄くし、上端に帯を出す", () => {
    const out = html(true);
    expect(out).toContain("前の結果");
    expect(out).toContain("refresh-bar");
    expect(out).toContain("opacity-55");
    expect(out).toContain('aria-busy="true"');
    expect(out).not.toContain("pointer-events-none opacity");
  });

  it("label を渡さない小さい場所（表・カード）では、ロゴを出さず、薄くするのと帯だけ", () => {
    const out = renderToStaticMarkup(
      <Refreshable active>
        <p>前の結果</p>
      </Refreshable>,
    );
    expect(out).toContain("opacity-55");
    expect(out).toContain("refresh-bar");
    expect(out.match(/pointer-events-none/g)).toHaveLength(1); // 帯だけ（中央のロゴの枠は無い）
  });

  it("操作を止めたいときだけ、中身を触れなくする", () => {
    expect(html(true, true)).toContain("pointer-events-none");
    expect(html(true, false).match(/pointer-events-none/g)).toHaveLength(2); // 帯と、中央のロゴを置く枠だけ
  });
});

describe("Settled（そろうまで待つ枠）", () => {
  const html = (pending: boolean) =>
    renderToStaticMarkup(
      <QueryClientProvider client={new QueryClient()}>
        <Settled pending={pending} label="熱を計算中">
          <p>中身</p>
        </Settled>
      </QueryClientProvider>,
    );

  it("データが無い間は中身を描かず、待ちの表示だけを出す", () => {
    const out = html(true);
    expect(out).toContain("熱を計算中");
    expect(out).not.toContain("中身");
  });

  it("データが来たら、見えない状態で中身を描いて（問い合わせを始めさせて）待つ", () => {
    const out = html(false);
    expect(out).toContain("熱を計算中");
    expect(out).toContain("invisible");
    expect(out).toContain("中身");
  });
});

describe("refreshingDim（枠にできない要素を薄くする class）", () => {
  it("読み直している間だけ薄くし、枠（Refreshable）と同じ度合いにする", () => {
    expect(refreshingDim(false)).toBe("");
    expect(refreshingDim(true)).toContain("opacity-55");
    expect(
      renderToStaticMarkup(
        <Refreshable active>
          <p />
        </Refreshable>,
      ),
    ).toContain("opacity-55");
  });
});
