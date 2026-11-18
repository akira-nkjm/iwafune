import { describe, expect, it } from "vitest";
import type { DesignView, NetView, PartRow } from "@/api/types";
import { fixture } from "@/test/fixture";
import { buildModel, cardNodeId, groupNodeId } from "./model";
import type { Layer } from "./power";

const both = new Set<Layer>(["power", "signal"]);

/** RW を 3 台（同じカタログ・同じ接続）にした設計 */
function withWheels(): DesignView {
  const rw = fixture.parts[0] as PartRow;
  const wheels = ["RW_X", "RW_Y", "RW_Z"].map((id) => ({ ...rw, id, catalog: "lab/rw_18mNms" }));
  const members = (n: string) =>
    fixture.nets.find((x) => x.id === n)?.members.filter((m) => m.part !== "RW_X") ?? [];
  return {
    ...fixture,
    parts: [...wheels, ...fixture.parts.slice(1)],
    nets: fixture.nets.map((n) => ({
      ...n,
      members: [
        ...members(n.id),
        ...wheels.map((w) => ({ part: w.id, port: n.kind === "power" ? "PWR" : "DATA" })),
      ],
    })),
  };
}

describe("系統図のモデル", () => {
  it("サブシステムの枠の中に部品のカード", () => {
    const m = buildModel(fixture, { layers: both, mergeSiblings: false });
    expect(m.containers.map((c) => c.id)).toEqual([
      groupNodeId("aocs"),
      groupNodeId("power"),
      groupNodeId("comm"),
    ]);
    expect(m.cards.find((c) => c.id === cardNodeId("PCDU"))?.parent).toBe(groupNodeId("power"));
  });

  it("電源は供給元から、データはマスターから星形に辺を出す", () => {
    const m = buildModel(fixture, { layers: both, mergeSiblings: false });
    const power = m.edges.filter((e) => e.layer === "power");
    expect(power.every((e) => e.source.node === cardNodeId("PCDU"))).toBe(true);
    expect(power.map((e) => e.target.node).sort()).toEqual([cardNodeId("RW_X"), cardNodeId("TRX")]);
    const signal = m.edges.filter((e) => e.layer === "signal");
    expect(signal.every((e) => e.label === "CAN")).toBe(true);
  });

  it("中心は API の hub で決める（役割名は見ない）。collects なら線は各端から中心へ", () => {
    const base = fixture.nets.find((n) => n.kind === "power") as NetView;
    const [hub, ...rest] = base.members;
    if (!hub) throw new Error("no members");
    const edgesOf = (collects: boolean) =>
      buildModel(
        { ...fixture, nets: [{ ...base, source: null, hub: { port: hub, collects } }] },
        { layers: both, mergeSiblings: false },
      ).edges;
    const out = edgesOf(false);
    expect(out.every((e) => e.source.node === cardNodeId(hub.part))).toBe(true);
    const into = edgesOf(true);
    expect(into.every((e) => e.target.node === cardNodeId(hub.part))).toBe(true);
    expect(into.length).toBe(rest.length);
    const none = buildModel(
      { ...fixture, nets: [{ ...base, source: null, hub: null }] },
      { layers: both, mergeSiblings: false },
    );
    expect(none.junctions.map((j) => j.net)).toEqual(rest.length > 1 ? [base.id] : []);
  });

  it("層を切り替えると、その層の辺だけ（凡例も）", () => {
    const m = buildModel(fixture, { layers: new Set<Layer>(["signal"]), mergeSiblings: false });
    expect(m.edges.every((e) => e.layer === "signal")).toBe(true);
    expect(m.legend).toEqual([]);
  });

  it("同種をまとめると、同じカタログ・同じ接続の兄弟は ×N の 1 枚になり、辺もまとまる", () => {
    const d = withWheels();
    const merged = buildModel(d, { layers: both, mergeSiblings: true });
    const card = merged.cards.find((c) => c.members.length === 3);
    expect(card?.label).toBe("RW");
    expect(card?.count).toBe(3);
    expect(
      merged.edges.filter((e) => e.target.node === card?.id && e.layer === "power"),
    ).toHaveLength(1);
    const apart = buildModel(d, { layers: both, mergeSiblings: false });
    expect(apart.cards.filter((c) => c.label.startsWith("RW_"))).toHaveLength(3);
  });

  it("同じ 2 枚の間の同じプロトコルの辺は 1 本にまとめて ×N", () => {
    const d: DesignView = {
      ...fixture,
      nets: [
        ...fixture.nets,
        {
          ...(fixture.nets[1] as DesignView["nets"][number]),
          id: "CAN2",
          master: { part: "PCDU", port: "CAN1" },
          members: [
            { part: "PCDU", port: "CAN1" },
            { part: "TRX", port: "CAN" },
          ],
        },
      ],
    };
    const m = buildModel(d, { layers: new Set<Layer>(["signal"]), mergeSiblings: false });
    const e = m.edges.find((x) => x.target.node === cardNodeId("TRX"));
    expect(e?.label).toBe("CAN×2");
    expect(e?.nets).toEqual(["CAN", "CAN2"]);
  });
});

it("カードも枠も名前ではなく API の系で色を決める", () => {
  const design = {
    ...fixture,
    parts: fixture.parts.map((p) => ({ ...p, system: "payload" as const })),
    tree: fixture.tree.map((n) => ({ ...n, system: "payload" as const })),
  };
  const model = buildModel(design, { layers: both, mergeSiblings: false });
  expect(model.cards.every((c) => c.tone === "rose")).toBe(true);
  expect(model.containers.every((c) => c.tone === "rose")).toBe(true);
});
