import { describe, expect, it } from "vitest";
import { idWithName, nodeName } from "./design";

describe("nodeName / idWithName", () => {
  const mode = { id: "CHARGE", label: "充電" };

  it("日本語は設計に書いた表示名、英語は ID", () => {
    expect(nodeName(mode, "ja")).toBe("充電");
    expect(nodeName(mode, "en")).toBe("CHARGE");
  });

  it("表示名が無ければどちらの言語でも ID", () => {
    expect(nodeName({ id: "AOCS", label: null }, "ja")).toBe("AOCS");
    expect(nodeName({ id: "AOCS" }, "en")).toBe("AOCS");
  });

  it("選択肢は「ID（表示名）」、表示名が ID と同じなら ID だけ", () => {
    expect(idWithName(mode, "ja")).toBe("CHARGE（充電）");
    expect(idWithName(mode, "en")).toBe("CHARGE");
    expect(idWithName({ id: "SAFE", label: "SAFE" }, "ja")).toBe("SAFE");
  });
});
