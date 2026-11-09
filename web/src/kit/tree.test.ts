import { describe, expect, it } from "vitest";
import type { PartRow } from "@/api/types";
import { catalogTree, filterTree, partsTree, treeCount } from "./tree";

type Part = Pick<PartRow, "id" | "parent" | "dir" | "subsystem">;
/** `dir` は API の PartRow.dir（`parts/` からのディレクトリの区間） */
const part = (id: string, dir: string[], parent: string | null = null): Part => ({
  id,
  dir,
  parent,
  subsystem: "POWER",
});

describe("partsTree", () => {
  it("groups by nested file folders rather than subsystem", () => {
    const nodes = partsTree([part("BAT", ["POWER", "batteries"]), part("RW", ["AOCS"])]);
    expect(nodes.map((n) => n.label)).toEqual(["POWER", "AOCS"]);
    expect(nodes[0]!.children[0]!.label).toBe("batteries");
    expect(nodes[0]!.children[0]!.children[0]!.item?.id).toBe("BAT");
    expect(treeCount(nodes[0]!)).toBe(1);
  });
  it("nests children below their assembly regardless of input order or file folder", () => {
    const nodes = partsTree([part("CELL", ["other"], "BAT"), part("BAT", ["POWER"], "GROUP")]);
    expect(nodes).toHaveLength(1);
    expect(nodes[0]!.children[0]!.item?.id).toBe("BAT");
    expect(nodes[0]!.children[0]!.children[0]!.item?.id).toBe("CELL");
    expect(treeCount(nodes[0]!)).toBe(2);
  });
  it("handles root files, missing parents, self references and cycles without losing parts", () => {
    const nodes = partsTree([
      part("A", [], "B"),
      part("B", [], "A"),
      part("C", [], "C"),
      part("D", [], "missing"),
    ]);
    expect(nodes[0]!.label).toBe("POWER");
    expect(nodes[0]!.children.map((n) => n.item?.id)).toEqual(["A", "B", "C", "D"]);
    expect(partsTree([])).toEqual([]);
  });
});

describe("catalogTree", () => {
  it("groups items by the catalog name from the API and separates folder keys from item IDs", () => {
    const nodes = catalogTree([
      { id: "local/bat_2s2p", scope: "local", name: "bat_2s2p", kind: "battery" },
      { id: "lab/rw", scope: "lab", name: "rw", kind: "wheel" },
    ]);
    expect(nodes.map((n) => n.label)).toEqual(["local", "lab"]);
    expect(nodes[0]!.children[0]!.label).toBe("bat_2s2p");
    expect(nodes[0]!.key).not.toBe(nodes[0]!.children[0]!.key);
  });
  it("groups items without a catalog name by kind", () => {
    const nodes = catalogTree([
      { id: "bat", scope: "", name: "bat", kind: "battery" },
      { id: "bat2", scope: "", name: "bat2", kind: "battery" },
      { id: "rw", scope: "", name: "rw", kind: "wheel" },
    ]);
    expect(nodes.map((n) => n.label)).toEqual(["battery", "wheel"]);
    expect(treeCount(nodes[0]!)).toBe(2);
  });
});

describe("filterTree", () => {
  const nodes = partsTree([
    part("BAT", ["POWER"]),
    part("CELL", ["POWER"], "BAT"),
    part("RW", ["AOCS"]),
  ]);
  it("retains ancestors and expands the path to a matching descendant without mutating the tree", () => {
    const filtered = filterTree(nodes, (p) => p.id === "CELL");
    expect(filtered.nodes.map((n) => n.label)).toEqual(["POWER"]);
    expect(filtered.nodes[0]!.children[0]!.children[0]!.item?.id).toBe("CELL");
    expect([...filtered.expanded].sort()).toEqual(["folder/POWER", "item/BAT"]);
    expect(nodes).toHaveLength(2);
  });
  it("removes unmatched descendants when only their parent matches", () => {
    const filtered = filterTree(nodes, (p) => p.id === "BAT");
    expect(filtered.nodes[0]!.children[0]!.children).toEqual([]);
  });
  it("returns an empty tree and expansion set when nothing matches", () => {
    expect(filterTree(nodes, () => false)).toEqual({ nodes: [], expanded: new Set() });
  });
});
