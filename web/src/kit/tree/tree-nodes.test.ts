import { describe, expect, it } from "vitest";
import type { PartRow, System } from "@/api/types";
import {
  ancestorKeys,
  branchKeys,
  catalogTree,
  filterTree,
  partsTree,
  treeCount,
  treeItems,
} from "./tree-nodes";

type Part = Pick<PartRow, "id" | "parent" | "dir" | "system">;
/** `dir` は API の PartRow.dir（`parts/` からのディレクトリの区間） */
const part = (id: string, dir: string[], parent: string | null = null): Part => ({
  id,
  dir,
  parent,
  system: "power",
});

describe("partsTree", () => {
  it("groups by nested file folders rather than subsystem", () => {
    const nodes = partsTree([part("BAT", ["power", "batteries"]), part("RW", ["aocs"])]);
    expect(nodes[0]!.label).toBe("power");
    expect(nodes[1]!.label).toBe("aocs");
    expect(nodes).toHaveLength(2);
    expect(nodes[0]!.children[0]!.label).toBe("batteries");
    expect(nodes[0]!.children[0]!.children[0]!.item?.id).toBe("BAT");
    expect(treeCount(nodes[0]!)).toBe(1);
  });
  it("nests children below their assembly regardless of input order or file folder", () => {
    const nodes = partsTree([part("CELL", ["other"], "BAT"), part("BAT", ["power"], "GROUP")]);
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
    expect(nodes[0]!.label).toBe("power");
    expect(nodes[0]!.children.map((n) => n.item?.id)).toEqual(["A", "B", "C", "D"]);
    expect(partsTree([])).toEqual([]);
  });
});

describe("catalogTree", () => {
  const labels = {
    unset: "未設定",
    kind: (id: string) => `K:${id}`,
    maturity: (v: string) => `M:${v}`,
    system: (id: string) => `S:${id}`,
    noTags: "タグなし",
    unassigned: "系なし",
  };
  const kinds = [
    { id: "device", parent: null },
    { id: "actuator", parent: "device" },
    { id: "wheel", parent: "actuator" },
    { id: "battery", parent: "device" },
  ];
  const item = (
    id: string,
    kind: string,
    manufacturer: string | null = null,
    maturity: "new" | "modified" | null = null,
    scope = "local",
    tags: string[] = [],
    system: System | null = null,
  ) => ({ id, scope, name: id, kind, manufacturer, maturity, tags, system });
  const items = [
    item("local-bat", "battery", "Acme", "new"),
    item("lab-rw", "wheel", "Acme", null, "lab"),
    item("lab-gyro", "gyro", null, null, "lab"),
  ];
  const tree = (by: Parameters<typeof catalogTree>[1]) => catalogTree(items, by, kinds, labels);

  it("catalog: カタログ名（scope）ごと。フォルダーの key は品の key と別", () => {
    const nodes = tree("catalog");
    expect(nodes.map((n) => n.label)).toEqual(["local", "lab"]);
    expect(nodes[0]!.children[0]!.label).toBe("local-bat");
    expect(nodes[0]!.key).not.toBe(nodes[0]!.children[0]!.key);
  });

  it("manufacturer: メーカー → 種別。メーカー無しは未設定で末尾", () => {
    const nodes = catalogTree([items[2]!, items[0]!, items[1]!], "manufacturer", kinds, labels);
    expect(nodes.map((n) => n.label)).toEqual(["Acme", "未設定"]);
    expect(nodes[0]!.children.map((n) => n.label)).toEqual(["K:battery", "K:wheel"]);
    expect(nodes[1]!.children[0]!.label).toBe("K:gyro");
    expect(treeCount(nodes[0]!)).toBe(2);
  });

  it("kind: 種別定義の親子を入れ子にし、品は自身の種別の下。定義の無い種別は単独", () => {
    const nodes = tree("kind");
    expect(nodes.map((n) => n.label)).toEqual(["K:device", "K:gyro"]);
    const device = nodes[0]!;
    expect(device.children.map((n) => n.label)).toEqual(["K:battery", "K:actuator"]);
    const actuator = device.children[1]!;
    expect(actuator.children[0]!.label).toBe("K:wheel");
    expect(actuator.children[0]!.children[0]!.item?.id).toBe("lab-rw");
  });

  it("kind: 親の循環で止まらない", () => {
    const cyclic = [
      { id: "a", parent: "b" },
      { id: "b", parent: "a" },
    ];
    const nodes = catalogTree([item("x-1", "a")], "kind", cyclic, labels);
    expect(treeItems(nodes[0]!)).toHaveLength(1);
  });

  it("maturity: 表示名のフォルダー。成熟度無しは未設定", () => {
    const nodes = tree("maturity");
    expect(nodes.map((n) => n.label)).toEqual(["M:new", "未設定"]);
    expect(treeCount(nodes[1]!)).toBe(2);
  });

  it("tag: 複数のタグを持つ品は各タグの下に出る。タグ無しは「タグなし」で末尾", () => {
    const tagged = [
      item("a", "wheel", null, null, "local", ["x", "y"]),
      item("b", "wheel", null, null, "local", []),
      item("c", "wheel", null, null, "local", ["y", "y"]),
    ];
    const nodes = catalogTree(tagged, "tag", kinds, labels);
    expect(nodes.map((n) => n.label)).toEqual(["x", "y", "タグなし"]);
    expect(nodes.map((n) => treeItems(n).map((i) => i.id))).toEqual([["a"], ["a", "c"], ["b"]]);
    const keys = nodes.flatMap((n) => n.children.map((c) => c.key));
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("system: 系の表示名のフォルダー。系の無い品は「系なし」で末尾", () => {
    const systemed = [
      item("a", "component"),
      item("b", "wheel", null, null, "local", [], "aocs"),
      item("c", "battery", null, null, "local", [], "power"),
    ];
    const nodes = catalogTree(systemed, "system", kinds, labels);
    expect(nodes.map((n) => n.label)).toEqual(["S:aocs", "S:power", "系なし"]);
    expect(treeItems(nodes[2]!).map((i) => i.id)).toEqual(["a"]);
  });

  it("フォルダーの key は分け方ごとに一意（区切りの文字を含む値でも混ざらない）", () => {
    const a = catalogTree(
      [item("p-1", "wheel", "a/b"), item("p-2", "wheel", "a")],
      "manufacturer",
      kinds,
      labels,
    );
    const keys: string[] = [];
    const walk = (ns: typeof a) => {
      for (const n of ns) {
        keys.push(n.key);
        walk(n.children);
      }
    };
    walk(a);
    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe("filterTree", () => {
  const nodes = partsTree([
    part("BAT", ["power"]),
    part("CELL", ["power"], "BAT"),
    part("RW", ["aocs"]),
  ]);
  it("retains ancestors and expands the path to a matching descendant without mutating the tree", () => {
    const filtered = filterTree(nodes, (p) => p.id === "CELL");
    expect(filtered.nodes.map((n) => n.label)).toEqual(["power"]);
    expect(filtered.nodes[0]!.children[0]!.children[0]!.item?.id).toBe("CELL");
    expect([...filtered.expanded].sort()).toEqual(["folder/power", "item/BAT"]);
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

describe("treeItems and ancestorKeys", () => {
  const nodes = partsTree([
    part("BAT", ["power", "batteries"]),
    part("CELL", ["other"], "BAT"),
    part("RW", ["aocs"]),
  ]);
  it("collects the item and all descendants of a node", () => {
    expect(treeItems(nodes[0]!).map((p) => p.id)).toEqual(["BAT", "CELL"]);
    expect(treeItems(nodes[1]!).map((p) => p.id)).toEqual(["RW"]);
  });
  it("returns the folder and assembly keys above a matching item, root first", () => {
    const power = nodes[0]!;
    const batteries = power.children[0]!;
    const bat = batteries.children[0]!;
    expect(ancestorKeys(nodes, (p) => p.id === "CELL")).toEqual([
      power.key,
      batteries.key,
      bat.key,
    ]);
    expect(ancestorKeys(nodes, (p) => p.id === "RW")).toEqual([nodes[1]!.key]);
    expect(ancestorKeys(nodes, (p) => p.id === "NONE")).toEqual([]);
  });
});

describe("branchKeys", () => {
  it("collects every node that has children at every depth, not leaves", () => {
    const nodes = partsTree([
      part("BAT", ["power", "batteries"]),
      part("CELL", ["other"], "BAT"),
      part("RW", ["aocs"]),
    ]);
    expect(branchKeys(nodes)).toEqual([
      "folder/power",
      "folder/power/batteries",
      "item/BAT",
      "folder/aocs",
    ]);
  });
  it("is empty for a flat list", () => {
    expect(branchKeys([{ key: "item/A", label: "A", item: { id: "A" }, children: [] }])).toEqual(
      [],
    );
  });
});
