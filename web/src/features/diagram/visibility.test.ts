import { describe, expect, it } from "vitest";
import type { DesignView, System } from "@/api/types";
import { fixture } from "@/test/fixture";
import { buildModel, cardNodeId, groupNodeId } from "./model";
import { LAYERS } from "./power";

const options = { layers: new Set(LAYERS), mergeSiblings: false };
const build = (design: DesignView, hidden: System[]) =>
  buildModel(design, { ...options, hiddenSystems: new Set(hidden) });

describe("系別の表示", () => {
  it("カードと接する線、空の枠を隠し、残る線の端や編集キーを変えない", () => {
    const full = buildModel(fixture, options);
    const filtered = build(fixture, ["aocs"]);
    expect(filtered.cards.some((c) => c.members.includes("RW_X"))).toBe(false);
    expect(filtered.containers.some((c) => c.id === groupNodeId("aocs"))).toBe(false);
    expect(filtered.edges.length).toBeGreaterThan(0);
    expect(filtered.edges).toEqual(
      full.edges.filter(
        (e) => e.source.node !== cardNodeId("RW_X") && e.target.node !== cardNodeId("RW_X"),
      ),
    );
    expect(filtered.cards.some((c) => c.members.includes("TRX"))).toBe(true);
    expect(build(fixture, []).cards).toEqual(full.cards);
  });

  it("供給元・マスターを隠しても、残る端を勝手につなぎ替えない", () => {
    const model = build(fixture, ["power"]);
    expect(model.cards.some((c) => c.members.includes("PCDU"))).toBe(false);
    expect(model.edges).toEqual([]);
  });

  it("全系を隠しても系のない部品は表示する。設計データは変更しない", () => {
    const unknown = { ...fixture.parts[0]!, id: "UNKNOWN", parent: null, system: null };
    const design = { ...fixture, parts: [...fixture.parts, unknown] };
    const before = JSON.stringify(design);
    const model = build(design, design.defs.systems);
    expect(model.cards.map((c) => c.members)).toEqual([["UNKNOWN"]]);
    expect(model.containers).toEqual([]);
    expect(model.edges).toEqual([]);
    expect(JSON.stringify(design)).toBe(before);
    const empty = build(fixture, fixture.defs.systems);
    expect(empty.cards).toEqual([]);
    expect(empty.junctions).toEqual([]);
  });

  it("隠れた組立品の別系の子と枠を可視の祖先に置く", () => {
    const design: DesignView = {
      ...fixture,
      parts: fixture.parts.map((p) => (p.id === "RW_X" ? { ...p, parent: "PCDU" } : p)),
      tree: fixture.tree.map((n) => {
        if (n.id === "RW_X") return { ...n, parent: "PCDU" };
        if (n.id === "PCDU") return { ...n, children: ["RW_X"] };
        return n;
      }),
    };
    const model = build(design, ["power"]);
    expect(model.cards.find((c) => c.id === cardNodeId("RW_X"))?.parent).toBe(groupNodeId("power"));
    expect(model.containers.some((c) => c.id === groupNodeId("power"))).toBe(true);
  });

  it("ハブのないネットは残る分岐点と可視の端の線を保つ", () => {
    const design: DesignView = {
      ...fixture,
      parts: fixture.parts.map((p) => ({ ...p, role: null })),
      nets: fixture.nets.map((n) => ({ ...n, source: null, master: null, hub: null })),
    };
    const model = build(design, ["power"]);
    expect(model.junctions.length).toBeGreaterThan(0);
    expect(model.edges.length).toBeGreaterThan(0);
    const nodes = new Set([...model.cards, ...model.junctions].map((n) => n.id));
    expect(model.edges.every((e) => nodes.has(e.source.node) && nodes.has(e.target.node))).toBe(
      true,
    );
  });
});
