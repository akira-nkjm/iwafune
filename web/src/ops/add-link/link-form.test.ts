import { describe, expect, it } from "vitest";
import type { DesignView } from "@/api/types";
import { fixture } from "@/test/fixture";
import { bandOptions, emptyLink, linkAddCommand, transceiverOptions } from "./link-form";

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
