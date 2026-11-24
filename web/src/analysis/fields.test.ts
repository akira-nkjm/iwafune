import { afterEach, expect, it } from "vitest";
import { setLang } from "@/i18n";
import { fieldLabel } from "./fields";

afterEach(() => setLang("ja"));

it("同じキーが解析ごとに違う意味のときは、持ち主のキーを付けた表示名を先に引く", () => {
  setLang("en");
  expect(fieldLabel("origin")).toBe("Sequence start time");
  expect(fieldLabel("origin", "frequencies")).toBe("Source");
  // 持ち主ごとの表示名が無ければ、キーだけの表示名
  expect(fieldLabel("origin", "cases")).toBe("Sequence start time");
  expect(fieldLabel("burn_time", "maneuvers")).toBe("Burn time");
  // 辞書に無いキーはそのまま
  expect(fieldLabel("no_such_field")).toBe("no_such_field");
  setLang("ja");
  expect(fieldLabel("axis", "envelope")).toBe("軸");
  expect(fieldLabel("burn_time")).toBe("噴射時間");
});
