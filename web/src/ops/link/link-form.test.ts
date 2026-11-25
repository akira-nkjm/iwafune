import { describe, expect, it } from "vitest";
import type { DesignView, LinkView, Op } from "@/api/types";
import { fixture } from "@/test/fixture";
import {
  bandOptions,
  emptyLink,
  LINK_FIELDS,
  linkAddCommand,
  linkEditOps,
  linkFields,
  linkFieldText,
  linkRenamedTo,
  transceiverOptions,
  withCurrent,
} from "./link-form";

describe("回線の欄 → fune link add の引数", () => {
  const full = { id: " x_up ", direction: "up", part: "UHF_TRX", band: "uhf" };

  it("4 つの欄が揃うまでは送らず、揃ったら打った文字列をそのまま渡す（読むのは service）", () => {
    expect(linkAddCommand(emptyLink())).toBeNull();
    for (const key of ["id", "direction", "part", "band"] as const) {
      expect(linkAddCommand({ ...full, [key]: "" })).toBeNull();
    }
    expect(linkAddCommand(full)).toEqual({ command: "link_add", ...full });
  });

  it("足りない入力が名指しした ID を欄の初期値にする", () => {
    expect(emptyLink("s_down").id).toBe("s_down");
    expect(emptyLink(null).id).toBe("");
  });

  it("送受信機と帯域の選択肢は API の add_choices から作る（部品は製品ツリーの名前を添える）", () => {
    const design: DesignView = {
      ...fixture,
      tree: [{ ...fixture.tree[0], id: "UHF_TRX", label: "UHF transceiver" } as never],
      add_choices: {
        transceivers: ["STX", "UHF_TRX"],
        bands: ["s_band", "uhf"],
        thermal_cases: [],
      },
    };
    expect(transceiverOptions(design).map((o) => o.value)).toEqual(["STX", "UHF_TRX"]);
    expect(transceiverOptions(design)[0]?.label).toBe("STX");
    expect(bandOptions(design.add_choices)).toEqual([
      { value: "s_band", label: "s_band" },
      { value: "uhf", label: "uhf" },
    ]);
  });
});

describe("開いた回線の直し", () => {
  const view: LinkView = {
    id: "s_down",
    direction: "down",
    part: "STX",
    band: "s_band",
    file: "analysis/comm.toml",
    group: "analysis.comm.link.s_down",
  };
  const set = (path: string): Op => ({ op: "set_setting", path, value: null });

  it("今の回線の値を欄の初期値にする", () => {
    expect(linkFields(view)).toEqual({
      id: "s_down",
      direction: "down",
      part: "STX",
      band: "s_band",
    });
  });

  it("直した欄だけ、API が返した回線の設定のパスに `fune set` と同じ文字列で書く", () => {
    const next = { ...linkFields(view), part: " UHF_TRX ", band: "s_band" };
    expect(linkFieldText(view, next, "part")).toMatchObject({
      path: "analysis.comm.link.s_down.part",
      value: "UHF_TRX",
      item: null,
    });
    expect(linkFieldText(view, next, "band")).toBeNull();
    expect(linkFieldText(view, { ...next, direction: "" }, "direction")).toBeNull();
  });

  it("欄の書き換えのあとに名前の変更を置く（欄は今の名前で書く）", () => {
    const next = { ...linkFields(view), id: " s_dl ", direction: "up" };
    const fieldOps = [set("direction"), null, null];
    expect(linkRenamedTo(view, next)).toBe("s_dl");
    expect(linkEditOps(view, next, fieldOps)).toEqual([
      set("direction"),
      { op: "rename_comm_link", id: "s_down", to: "s_dl" },
    ]);
    // 直した欄の Op がまだ届いていない間は確定できない
    expect(linkEditOps(view, next, [null, null, null])).toBeNull();
  });

  it("何も直していなければ Op は無く、ID だけ直せば名前の変更だけ", () => {
    expect(linkEditOps(view, linkFields(view), [null, null, null])).toBeNull();
    expect(linkEditOps(view, { ...linkFields(view), id: "s_dl" }, [null, null, null])).toEqual([
      { op: "rename_comm_link", id: "s_down", to: "s_dl" },
    ]);
    expect(LINK_FIELDS).toEqual(["direction", "part", "band"]);
  });

  it("選択肢に無い今の値は、黙って変えないよう先頭に足す", () => {
    const options = [{ value: "uhf", label: "uhf" }];
    expect(withCurrent(options, "s_band")[0]).toEqual({ value: "s_band", label: "s_band" });
    expect(withCurrent(options, "uhf")).toBe(options);
    expect(withCurrent(options, null)).toBe(options);
  });
});
