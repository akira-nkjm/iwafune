import { describe, expect, it } from "vitest";
import type { PhaseView } from "@/api/types";
import { qv } from "@/test/fixture";
import {
  addableModes,
  copyOrbitCommand,
  lawKindPath,
  overrideRows,
  unsetLawCommand,
  unsetOrbitCommand,
} from "./override-commands";

describe("フェーズの上書きのコマンド", () => {
  it("fune phase copy-orbit・unset-orbit・unset-law と同じ引数にする", () => {
    expect(copyOrbitCommand("LEOP")).toEqual({ command: "phase_copy_orbit", phase: "LEOP" });
    expect(unsetOrbitCommand("LEOP")).toEqual({ command: "phase_unset_orbit", phase: "LEOP" });
    expect(unsetLawCommand("LEOP", "SAFE")).toEqual({
      command: "phase_unset_law",
      phase: "LEOP",
      mode: "SAFE",
    });
  });

  it("姿勢則の上書きは、種類の欄（law.<モード>.law）から始める", () => {
    expect(lawKindPath("SAFE")).toBe("law.SAFE.law");
  });
});

describe("overrideRows", () => {
  it("欄のパス・値・表示をそのまま設定の表の行にする", () => {
    const value = {
      type: "scalar" as const,
      nominal: qv(250, "km", "Length", 250000),
      min: null,
      max: null,
    };
    const rows = overrideRows(
      [{ path: "orbit.altitude", key: "altitude", display: "250 km", value, text: "250 km" }],
      "phases/DISPOSAL.toml",
    );
    expect(rows).toEqual([
      {
        key: "orbit.altitude",
        field: "altitude",
        value,
        display: "250 km",
        file: "phases/DISPOSAL.toml",
      },
    ]);
  });
});

describe("addableModes", () => {
  it("すでに上書きしたモードは選べない", () => {
    const phase = { laws: [{ mode: "SAFE" }] } as unknown as PhaseView;
    expect(addableModes(["SAFE", "NOMINAL"], phase)).toEqual(["NOMINAL"]);
  });
});
