import { describe, expect, it } from "vitest";
import { numParts, numText } from "./chart";

describe("numParts / numText", () => {
  it("同じグラフの最大に比べて丸めの残りしかない値は 0", () => {
    expect(numText(2.135e-16, 19.2)).toBe("0");
    expect(numText(0)).toBe("0");
  });
  it("小さすぎる値と大きすぎる値は指数", () => {
    expect(numParts(0.000214, 0.0005)).toEqual({ kind: "exp", mantissa: "2.14", exponent: -4 });
    expect(numText(-0.000214, 0.0005)).toBe("-2.14e-4");
    expect(numText(3_200_000, 3_200_000)).toBe("3.2e6");
  });
  it("ふつうの値は有効数字 4 桁", () => {
    expect(numText(19.2234)).toBe("19.22");
    expect(numText(0.0123)).toBe("0.0123");
  });
});
