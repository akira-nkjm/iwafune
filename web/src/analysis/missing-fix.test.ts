import { expect, it } from "vitest";
import type { DesignView, Fix, ManeuverView, MissingInput } from "@/api/types";
import { choiceLabel } from "@/kit/forms/choice-label";
import { fixture } from "@/test/fixture";
import { type PartFix, partSetText, pickEdit, rowActions, valueSetText } from "./missing-fix";

const valueFix: Extract<Fix, { kind: "set_value" }> = {
  kind: "set_value",
  path: "RW_X.max_momentum",
  target: { target: "attr", part: "RW_X", attr: "max_momentum", state: null },
  dimension: "AngularMomentum",
  command: 'fune set RW_X.max_momentum "<value>"',
  example: "1 mN*m*s",
  choices: [],
  catalog: {
    item: "local/rw_10mNms",
    path: "max_momentum",
    command: 'fune catalog set local/rw_10mNms max_momentum "<value>"',
  },
};
const pickFix: Extract<Fix, { kind: "pick_ref" }> = {
  kind: "pick_ref",
  what: "part",
  kinds: [],
  add_command: null,
  target: { kind: "maneuver", id: "RAISE", field: "thruster" },
  path: "maneuvers.RAISE.thruster",
  roles: ["propulsion_thruster", "thruster"],
  choices: [{ value: "EPT", label: "EPT", label_en: null }],
  command: 'fune maneuver set RAISE --category insertion --method low_thrust --thruster "<part>"',
};

const missing = (fixes: Fix[]): MissingInput => ({
  part: null,
  path: "x",
  target: null,
  reason: "r",
  fixes,
});

it("候補の種類ごとに、その場の編集か既存のページへの道を出す", () => {
  const actions = rowActions(
    missing([
      valueFix,
      pickFix,
      { kind: "add_part", roles: ["battery"], kinds: ["battery"], catalog_items: [], command: "" },
      { kind: "assign_part", roles: ["battery"], kinds: [], parts: ["OBC"], command: "" },
      {
        kind: "set_value_on_part",
        attr: "a",
        dimension: null,
        example: null,
        choices: [],
        parts: [],
        command: "",
      },
      { kind: "add_list_item", list: "mode", command: "fune mode add <ID>" },
    ]),
  );
  expect(actions.map((a) => a.kind)).toEqual(["value", "pick", "add_part", "part", "part", "link"]);
  const links = actions.flatMap((a) => (a.kind === "link" ? [a.to] : []));
  expect(links).toEqual(["/design/modes"]);
  expect(rowActions(missing([]))).toEqual([]);
});

it("値の書き込み先: 部品の上書きは部品のパス、カタログ品は品とその中のパスで送る", () => {
  expect(valueSetText(valueFix, "part", " 10 mN*m*s ")).toMatchObject({
    item: null,
    path: "RW_X.max_momentum",
    value: "10 mN*m*s",
  });
  expect(valueSetText(valueFix, "catalog", "10 mN*m*s")).toMatchObject({
    item: "local/rw_10mNms",
    path: "max_momentum",
  });
  // 品の書き先が無い候補では、品を選んでも部品の上書きになる。空の値は送らない
  expect(valueSetText({ ...valueFix, catalog: null }, "catalog", "1")?.item).toBeNull();
  expect(valueSetText(valueFix, "part", "  ")).toBeNull();
});

const maneuver = {
  id: "RAISE",
  text: { id: "RAISE", category: "insertion", method: "low_thrust", altitude: "600 km" },
} as unknown as ManeuverView;
const design: DesignView = { ...fixture, maneuvers: [maneuver] };

it("マヌーバのスラスタは `maneuver_set` に今の値のまま部品だけ置き換えて送る", () => {
  expect(pickEdit(pickFix, "EPT", design)).toEqual({
    command: {
      command: "maneuver_set",
      id: "RAISE",
      category: "insertion",
      method: "low_thrust",
      altitude: "600 km",
      thruster: "EPT",
    },
  });
  expect(pickEdit(pickFix, "", design)).toBeNull();
  expect(pickEdit(pickFix, "EPT", { ...design, maneuvers: [] })).toBeNull();
});

it("設定の部品の欄は、欄のパスに部品の ID を `from_text` で送る", () => {
  const fix: typeof pickFix = {
    ...pickFix,
    target: {
      kind: "param",
      target: "setting",
      scope: "analysis",
      segments: ["comm", "link", "uhf_down", "part"],
    },
    path: "analysis.comm.link.uhf_down.part",
  };
  expect(pickEdit(fix, "UHF_TRX", design)).toEqual({
    text: expect.objectContaining({
      item: null,
      path: "analysis.comm.link.uhf_down.part",
      value: "UHF_TRX",
    }),
  });
});

const launcherFix: typeof pickFix = {
  ...pickFix,
  what: "catalog_item",
  kinds: ["launcher"],
  target: {
    kind: "param",
    target: "setting",
    scope: "conditions",
    segments: ["launch", "launcher"],
  },
  path: "conditions.launch.launcher",
  choices: [
    { value: "local/lv_alpha", label: "Alpha", label_en: "Alpha LV" },
    { value: "local/lv_beta", label: "local/lv_beta", label_en: null },
  ],
  command: "fune set conditions.launch.launcher <item>",
  add_command: "fune catalog add local/<ID> --kind launcher",
};

it("カタログ品を選ぶと、欄のパスにカタログの参照を `from_text` で送る", () => {
  const rows = rowActions(missing([launcherFix]));
  expect(rows.map((a) => a.kind)).toEqual(["pick"]);
  expect(pickEdit(launcherFix, "local/lv_alpha", design)).toEqual({
    text: expect.objectContaining({
      item: null,
      path: "conditions.launch.launcher",
      value: "local/lv_alpha",
    }),
  });
  expect(pickEdit(launcherFix, "", design)).toBeNull();
});

it("選べるものの表示は、表示名があれば値を添え、無ければ値だけ", () => {
  expect(launcherFix.choices.map(choiceLabel)).toEqual(["Alpha (local/lv_alpha)", "local/lv_beta"]);
});

it("部品を選んで書く編集: 値を入れる候補は <部品>.<属性>、割り当てる候補は <部品>.kind に送る", () => {
  const set: PartFix = {
    kind: "set_value_on_part",
    attr: "max_momentum",
    dimension: "AngularMomentum",
    example: null,
    choices: [],
    parts: ["RW_X"],
    command: "",
  };
  const assign: PartFix = {
    kind: "assign_part",
    roles: ["battery"],
    kinds: ["battery"],
    parts: ["OBC"],
    command: "",
  };
  expect(partSetText(set, "RW_X", " 10 mN*m*s ")).toMatchObject({
    item: null,
    path: "RW_X.max_momentum",
    value: "10 mN*m*s",
  });
  expect(partSetText(assign, "OBC", "battery")).toMatchObject({
    path: "OBC.kind",
    value: "battery",
  });
  // 部品か値が空なら何も送らない
  expect(partSetText(set, "", "1")).toBeNull();
  expect(partSetText(assign, "OBC", "")).toBeNull();
});
