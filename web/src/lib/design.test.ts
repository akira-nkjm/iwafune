import { afterEach, describe, expect, it } from "vitest";
import { getLang, setLang } from "@/i18n";
import { fixture } from "@/test/fixture";
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
  parentOptions,
  systemTree,
  withNote,
} from "./design";

describe("withNote", () => {
  it("日本語は全角の括弧、英語は半角の括弧。補足が無いか主と同じなら主だけ", () => {
    expect(withNote("TRX", "transceiver", "ja")).toBe("TRX（transceiver）");
    expect(withNote("TRX", "transceiver", "en")).toBe("TRX (transceiver)");
    expect(withNote("TRX", "TRX", "ja")).toBe("TRX");
    expect(withNote("TRX", null, "ja")).toBe("TRX");
    expect(withNote("TRX", "", "en")).toBe("TRX");
  });
});

describe("nodeName / idWithName", () => {
  const mode = { id: "CHARGE", label: "充電" };

  it("日本語は設計に書いた表示名、英語は ID", () => {
    expect(nodeName(mode, "ja")).toBe("充電");
    expect(nodeName(mode, "en")).toBe("CHARGE");
  });

  it("表示名が無ければどちらの言語でも ID", () => {
    expect(nodeName({ id: "aocs", label: null }, "ja")).toBe("aocs");
    expect(nodeName({ id: "aocs" }, "en")).toBe("aocs");
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

describe("系別のツリーと置き場所", () => {
  it("API の系で直下の部品をまとめ、元の置き場所を変えない", () => {
    const part = { ...fixture.parts[0]!, parent: null, dir: [] };
    const design = {
      ...fixture,
      parts: [part],
      tree: [{ ...fixture.tree.find((n) => n.id === part.id)!, parent: null }],
    };
    const tree = systemTree(design);
    expect(tree.find((n) => n.id === "aocs")?.children).toEqual([part.id]);
    expect(tree.find((n) => n.id === part.id)?.parent).toBe("aocs");
    expect(design.tree[0]?.parent).toBeNull();
    expect(part.dir).toEqual([]);
  });
  it("未所属の部品を系に割り当てず、未作成の系も選べる", () => {
    const part = { ...fixture.parts[0]!, parent: null, system: null, dir: [] };
    const design = {
      ...fixture,
      parts: [part],
      tree: [{ ...fixture.tree.find((n) => n.id === part.id)!, parent: null, system: null }],
    };
    expect(systemTree(design)).toHaveLength(1);
    expect(parentOptions(design).some((o) => o.value === "payload")).toBe(true);
  });
  it("系の表示名は API の語彙の日英名を使う", () => {
    const node = { id: "payload", label: "ミッション機器", label_en: "Payload" };
    expect(nodeName(node, "ja")).toBe("ミッション機器");
    expect(nodeName(node, "en")).toBe("Payload");
  });
});
