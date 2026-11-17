import { describe, expect, it } from "vitest";
import type { CatalogItemView, DesignView, PartRow } from "@/api/types";
import { fixture } from "@/test/fixture";
import { filterPickOptions, groupForSystem, type PickOption, pickOptions } from "./catalog-pick";

const options: PickOption[] = [
  { id: "lab/rw_10", label: "Reaction wheel 10", note: "Reaction wheel · Acme" },
  { id: "local/tank", label: "Tank", note: "Tank" },
];

describe("カタログの品を選ぶ一覧", () => {
  it("打った文字で ID・表示名・補足を絞る（大文字小文字は区別せず、空白区切りはすべて含む）", () => {
    expect(filterPickOptions(options, "").map((o) => o.id)).toEqual(["lab/rw_10", "local/tank"]);
    expect(filterPickOptions(options, "ACME").map((o) => o.id)).toEqual(["lab/rw_10"]);
    expect(filterPickOptions(options, "wheel 10").map((o) => o.id)).toEqual(["lab/rw_10"]);
    expect(filterPickOptions(options, "wheel tank")).toEqual([]);
  });

  it("系が置き場の選択肢にあるときだけ、その系を返す", () => {
    const groups = [{ value: "aocs" }, { value: "BUS" }];
    expect(groupForSystem(groups, "aocs")).toBe("aocs");
    expect(groupForSystem(groups, "power")).toBeNull();
    expect(groupForSystem(groups, null)).toBeNull();
  });

  it("カタログに載っていない参照先も選べ、ID 順に並ぶ", () => {
    const item = { id: "local/b", kind: "x", label: "B", label_en: "B", manufacturer: null };
    const part = { candidates: ["lab/z"], catalog: "local/b" } as unknown as PartRow;
    const design = {
      ...fixture,
      catalog: [item as unknown as CatalogItemView],
      parts: [part],
    } as DesignView;
    expect(pickOptions(design).map((o) => o.id)).toEqual(["lab/z", "local/b"]);
    expect(pickOptions(design)[1]?.label).toBe("B");
  });
});
