import { describe, expect, it } from "vitest";
import { clearFieldErrors, fieldError, setFieldErrors } from "./field-errors";

describe("欄ごとのエラー", () => {
  it("設定して消せる（同じパスは上書き）", () => {
    setFieldErrors([{ path: "RW_X.mass", message: "次元が違います" }]);
    setFieldErrors([{ path: "RW_X.mass", message: "許可されない単位です" }]);
    expect(fieldError("RW_X.mass")).toBe("許可されない単位です");
    clearFieldErrors(["RW_X.mass"]);
    expect(fieldError("RW_X.mass")).toBeNull();
    expect(fieldError(null)).toBeNull();
  });
});
