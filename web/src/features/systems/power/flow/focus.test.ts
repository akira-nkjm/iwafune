import { describe, expect, it } from "vitest";
import { smallFlow } from "@/test/flow";
import { focusOf, neighbours } from "./focus";
import { foldEdges } from "./layout";

const edges = foldEdges(smallFlow().edges, new Map());

describe("focusOf", () => {
  it("何も載せていなければ null", () => {
    expect(focusOf(edges, null)).toBeNull();
  });

  it("カードならその出入りの線・両端のカード・始点の行", () => {
    const f = focusOf(edges, { kind: "card", id: "analysis:balance" });
    expect(f?.cards.has("part:SP")).toBe(true);
    expect(f?.cards.has("analysis:load")).toBe(true);
    expect(f?.edges.has("analysis:load>analysis:balance")).toBe(true);
    expect([...(f?.rows.get("part:SP") ?? [])].sort()).toEqual(["area", "eol_factor"]);
    // 負荷 → 消費電力だけの線は強調しない
    expect(f?.edges.has("part:L1#power>analysis:load")).toBe(false);
  });

  it("行ならその行の線だけ", () => {
    const f = focusOf(edges, { kind: "row", card: "part:SP", row: "area" });
    expect([...(f?.edges ?? [])]).toEqual(["part:SP#area>analysis:balance"]);
    expect([...(f?.rows.get("part:SP") ?? [])]).toEqual(["area"]);
    expect(f?.cards.has("analysis:load")).toBe(false);
  });

  it("線ならその線と両端", () => {
    const f = focusOf(edges, { kind: "edge", id: "analysis:load>analysis:balance" });
    expect([...(f?.cards ?? [])].sort()).toEqual(["analysis:balance", "analysis:load"]);
    expect(f?.edges.size).toBe(1);
  });
});

describe("neighbours", () => {
  it("線を引くカードと引く先のカード", () => {
    const n = neighbours(edges, "analysis:balance");
    expect(n.from).toContain("analysis:load");
    expect(n.from).toContain("setting:analysis.power");
    expect(n.to).toEqual([]);
    expect(neighbours(edges, "part:L1").to.sort()).toEqual(["analysis:balance", "analysis:load"]);
  });
});
