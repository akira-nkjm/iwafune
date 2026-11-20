import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { MARK } from "@/shell/brand/brand-art";
import { Refreshable } from "./Refreshable";
import { Working, WorkingChip } from "./Working";

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
    expect(renderToStaticMarkup(<WorkingChip label="x" />)).toBe("");
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

  it("操作を止めたいときだけ、中身を触れなくする", () => {
    expect(html(true, true)).toContain("pointer-events-none");
    expect(html(true, false).match(/pointer-events-none/g)).toHaveLength(2); // 帯と、中央のロゴを置く枠だけ
  });
});
