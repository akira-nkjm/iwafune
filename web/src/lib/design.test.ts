import { afterEach, describe, expect, it } from "vitest";
import { getLang, setLang } from "@/i18n";
import {
  attrLabel,
  defName,
  defWithId,
  fieldPath,
  findParamPath,
  idWithName,
  kindLabel,
  nodeName,
  paramPath,
} from "./design";

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

describe("属性・種別の表示名", () => {
  const before = getLang();
  afterEach(() => setLang(before));
  const rw = { id: "reaction_wheel", label: "リアクションホイール", label_en: "Reaction wheel" };
  const defs = {
    kinds: [rw],
    attributes: [{ id: "mass", label: "質量", label_en: null }],
  } as unknown as Parameters<typeof kindLabel>[0] & Parameters<typeof attrLabel>[0];

  it("表示の言語の名前（英語名が無ければ日本語）を引き、定義が無ければ ID", () => {
    setLang("ja");
    expect(kindLabel(defs, "reaction_wheel")).toBe("リアクションホイール");
    expect(attrLabel(defs, "mass")).toBe("質量");
    setLang("en");
    expect(kindLabel(defs, "reaction_wheel")).toBe("Reaction wheel");
    expect(attrLabel(defs, "mass")).toBe("質量");
    expect(kindLabel(defs, "heater")).toBe("heater");
    expect(attrLabel(defs, "max_dipole")).toBe("max_dipole");
  });

  it("選択肢は「ID（表示名）」、表示名が無ければ ID", () => {
    setLang("ja");
    expect(defWithId(rw)).toBe("reaction_wheel（リアクションホイール）");
    expect(defName({ id: "x", label: "", label_en: null })).toBe("x");
    expect(defWithId({ id: "x", label: "", label_en: null })).toBe("x");
  });
});

describe("パラメーターのパス", () => {
  const part = {
    id: "RW_X",
    paths: { power: { "": "RW_X.power", on: "RW_X.power.on" } },
    field_paths: { count: "RW_X.count" },
  };

  it("定義のある属性は API の paths を引く", () => {
    expect(paramPath(part, "power", "on")).toBe("RW_X.power.on");
    expect(paramPath(part, "power")).toBe("RW_X.power");
    expect(findParamPath(part, "power", "on")).toBe("RW_X.power.on");
  });

  it("API の paths に無い属性・状態はつながない（null）", () => {
    expect(findParamPath(part, "tank_pressure")).toBeNull();
    expect(findParamPath(part, "power", "peak")).toBeNull();
  });

  it("部品のフィールドは API の field_paths を引く", () => {
    expect(fieldPath(part, "count")).toBe("RW_X.count");
    expect(fieldPath(part, "maturity")).toBeNull();
  });
});
