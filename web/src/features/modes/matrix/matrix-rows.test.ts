import { describe, expect, it } from "vitest";
import type { DesignView, PartRow, SystemModesView } from "@/api/types";
import { modesJa } from "@/i18n/modes/ja";
import { fixture, qv } from "@/test/fixture";
import { cellOf, draftCommand, type ModeDraft, withCell } from "../draft/draft";
import { toggleCell, unassignedCells } from "./matrix";
import { matrixGroups } from "./matrix-rows";

/** 送受信機にも状態ごとの電力を持たせた設計（定義では電力が無い PCDU も表示する） */
function design(): DesignView {
  const parts = fixture.parts.map((p): PartRow => {
    const base = p.values.tx_power;
    if (p.id !== "TRX" || !base) return p;
    const power = {
      type: "by_state" as const,
      states: { off: qv(0, "W", "Power"), rx: qv(2, "W", "Power"), tx: qv(5, "W", "Power") },
    };
    return {
      ...p,
      count: 2,
      values: { ...p.values, power: { ...base, attr: "power", value: power } },
    };
  });
  return { ...fixture, parts };
}

function load(mean: number): SystemModesView {
  const values = {
    parts: { TRX: qv(mean, "W", "Power") },
    total: null,
    available: null,
    margin: null,
    verdict: null,
  };
  return {
    system: "power",
    missing: [],
    summary: [],
    modes: [
      {
        id: "NOMINAL",
        sunlit: values,
        eclipse: {
          ...values,
          parts: { TRX: qv(1, "W", "Power") },
        },
        aocs: null,
      },
    ],
  };
}

describe("matrixGroups", () => {
  it("定義では電力を持たない部品も、サブシステムごとに ID の順で並べ、セルの位置の行を通しで振る", () => {
    const groups = matrixGroups(design(), new Map(), null, modesJa.modes);
    expect(groups.map((g) => [g.group, g.label, g.rows.map((r) => r.part)])).toEqual([
      ["aocs", "AOCS", ["RW_X"]],
      ["comm", "通信", ["TRX"]],
      ["power", "電源", ["PCDU"]],
    ]);
    // 系の見出しの行をまたいでも、矢印キーで上下に動けるよう行の番号は通し
    const at = groups.flatMap((g) => g.rows.flatMap((r) => r.cells.map((c) => c.at)));
    expect(at).toEqual([
      { row: 0, column: 0 },
      { row: 0, column: 1 },
      { row: 1, column: 0 },
      { row: 1, column: 1 },
      { row: 2, column: 0 },
      { row: 2, column: 1 },
    ]);
  });

  it("行には員数・電力レベル（小さい順。API の整形）と、モードの数だけのセルを持つ", () => {
    const trx = matrixGroups(design(), new Map(), null, modesJa.modes)[1]?.rows[0];
    expect(trx?.count).toBe(2);
    expect(trx?.levels).toBe("0 W / 2 W / 5 W");
    expect(trx?.levelsTitle).toBe("off: 0 W / rx: 2 W / tx: 5 W");
    expect(trx?.cells.map((c) => c.mode)).toEqual(["NOMINAL", "SAFE"]);
  });

  it("セルだけで描けるよう、部品と状態の語彙もセルに持つ", () => {
    const trx = matrixGroups(design(), new Map(), null, modesJa.modes)[1]?.rows[0];
    expect(trx?.cells.map((c) => [c.part, c.states])).toEqual([
      ["TRX", ["off", "rx", "tx"]],
      ["TRX", ["off", "rx", "tx"]],
    ]);
  });

  it("保存済みのセルは文言と濃さ、電力は下書きを当てた解析の出力から", () => {
    const [nominal, safe] =
      matrixGroups(design(), new Map(), load(2.3), modesJa.modes)[1]?.rows[0]?.cells ?? [];
    expect(nominal).toMatchObject({
      value: { state: "rx", duty: { tx: "0.1" } },
      display: "rx（tx 10 %）",
      changed: false,
      quantity: { si: 2.3, dimension: "Power" },
      shade: 1,
    });
    expect(safe).toMatchObject({ value: null, display: undefined, quantity: null, shade: -1 });
  });

  it("下書きのセルは下書きの値で、変えた印を付ける", () => {
    const draft: ModeDraft = new Map([["SAFE", new Map([["TRX", { state: "tx", duty: {} }]])]]);
    const safe = matrixGroups(design(), draft, null, modesJa.modes)[1]?.rows[0]?.cells[1];
    expect(safe).toMatchObject({
      value: { state: "tx", duty: {} },
      changed: true,
      display: undefined,
      shade: 2,
    });
  });
});

describe("unassignedCells", () => {
  it("部品 × モードのうち、状態の書かれていないセルを数える", () => {
    // RW_X は SAFE、TRX は SAFE が無い（PCDU は両方ある）
    expect(unassignedCells(fixture.modes)).toBe(2);
  });
});

it("日照／蝕で状態を切り替え、上書きしたセルだけを示す。解除後は日照の下書きを表示する", () => {
  const d = design();
  let draft = withCell(d.modes, new Map(), "NOMINAL", "TRX", { state: "tx", duty: {} }, true);
  const sunlit = matrixGroups(d, draft, load(2.3), modesJa.modes)[1]?.rows[0]?.cells[0];
  const eclipse = matrixGroups(d, draft, load(2.3), modesJa.modes, true)[1]?.rows[0];
  expect(sunlit).toMatchObject({
    value: { state: "rx" },
    override: false,
    quantity: { si: 2.3, dimension: "Power" },
  });
  expect(eclipse?.cells[0]).toMatchObject({
    value: { state: "tx" },
    override: true,
    eclipse: true,
    quantity: { si: 1, dimension: "Power" },
  });
  expect(eclipse?.cells[1]).toMatchObject({ value: null, override: false, eclipse: true });
  draft = withCell(d.modes, draft, "NOMINAL", "TRX", { state: "off", duty: {} });
  draft = withCell(d.modes, draft, "NOMINAL", "TRX", null, true);
  expect(matrixGroups(d, draft, null, modesJa.modes, true)[1]?.rows[0]?.cells[0]).toMatchObject({
    value: { state: "off" },
    override: false,
  });
});

it("蝕のセルをクリックで切り替えると、表示中の継承状態から蝕だけを書き換える", () => {
  const d = design();
  const cell = matrixGroups(d, new Map(), null, modesJa.modes, true)[1]?.rows[0]?.cells[0];
  expect(cell).toBeDefined();
  if (!cell) return;
  const draft = withCell(
    d.modes,
    new Map(),
    cell.mode,
    cell.part,
    toggleCell(cell.roles, cell.value),
    true,
  );
  expect(draftCommand(d.modes, draft)).toMatchObject({ cells: [{ state: "off", eclipse: true }] });
  expect(cellOf(d.modes, draft, cell.mode, cell.part)?.state).toBe("rx");
});

it("保存済みの蝕のセルを表示し、解除した直後は日照の整形済みの割合を引き継ぐ", () => {
  const source = design();
  const d = {
    ...source,
    modes: { ...source.modes, rows: source.modes.rows.map((r) => ({ ...r })) },
  };
  const row = d.modes.rows.find((r) => r.part === "TRX");
  if (!row) throw new Error("TRX fixture missing");
  row.eclipse_cells = { NOMINAL: { state: "tx", duty: { rx: 0.25 } } };
  const eclipse = matrixGroups(d, new Map(), null, modesJa.modes, true)[1]?.rows[0]?.cells[0];
  expect(eclipse).toMatchObject({
    value: { state: "tx", duty: { rx: "0.25" } },
    override: true,
    savedDuty: { rx: 0.25 },
    changed: false,
  });
  const cleared = withCell(d.modes, new Map(), "NOMINAL", "TRX", null, true);
  const inherited = matrixGroups(d, cleared, null, modesJa.modes, true)[1]?.rows[0]?.cells[0];
  expect(inherited).toMatchObject({
    value: { state: "rx" },
    display: "rx（tx 10 %）",
    override: false,
    changed: true,
  });
});

it("系別セルは API の次元を保持し、同じ mode_set の下書きで編集する", () => {
  const view: SystemModesView = {
    system: "cdh",
    missing: [],
    summary: [],
    modes: [
      {
        id: "NOMINAL",
        aocs: null,
        sunlit: {
          parts: { TRX: qv(8, "kbit/s", "DataRate") },
          total: null,
          available: null,
          margin: null,
          verdict: "ok",
        },
        eclipse: {
          parts: { TRX: qv(2, "kbit/s", "DataRate") },
          total: null,
          available: null,
          margin: null,
          verdict: "ok",
        },
      },
    ],
  };
  const cell = matrixGroups(design(), new Map(), view, modesJa.modes, true, "cdh")[0]?.rows[0]
    ?.cells[0];
  expect(cell?.quantity?.dimension).toBe("DataRate");
  expect(cell?.quantity?.display).toBe("2 kbit/s");
  if (!cell) throw new Error("missing cell");
  const draft = withCell(
    design().modes,
    new Map(),
    cell.mode,
    cell.part,
    toggleCell(cell.roles, cell.value),
    true,
  );
  expect(draftCommand(design().modes, draft)).toMatchObject({
    command: "mode_set",
    cells: [{ part: "TRX", mode: "NOMINAL", eclipse: true }],
  });
});
