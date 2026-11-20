import { describe, expect, it } from "vitest";
import type { PartRow, TreeNode } from "@/api/types";
import { fixture } from "@/test/fixture";
import { groupOps, siblingParts, ungroupOps, ungroupParts } from "./group-ops";

const node = (id: string, kind: "group" | "part", parent: string | null, children: string[] = []) =>
  ({ id, node: kind, label: id, label_en: null, system: null, parent, children }) as TreeNode;

describe("グループにまとめる Op", () => {
  it("部品ごとに、親を 1 つだけ連鎖に入れた move_part を作る", () => {
    expect(groupOps("aocs", "RW", ["RW_X", "RW_Y"])).toEqual([
      { op: "move_part", id: "RW_X", to: "RW", under: ["aocs"] },
      { op: "move_part", id: "RW_Y", to: "RW", under: ["aocs"] },
    ]);
  });

  it("グループを解く Op は親へ戻す（連鎖は空）", () => {
    expect(ungroupOps("aocs", ["RW_X"])).toEqual([
      { op: "move_part", id: "RW_X", to: "aocs", under: [] },
    ]);
  });
});

describe("まとめる部品の選択肢", () => {
  const part = (id: string, parent: string | null) => ({ id, parent }) as PartRow;
  const design = {
    ...fixture,
    parts: [part("A", "aocs"), part("B", "aocs"), part("C", "power"), part("D", null)],
  };

  it("同じ親の部品だけを出す", () => {
    expect(siblingParts(design, part("A", "aocs")).map((p) => p.id)).toEqual(["A", "B"]);
  });

  it("親が無い部品はまとめられない", () => {
    expect(siblingParts(design, part("D", null))).toEqual([]);
  });
});

describe("グループを解けるか", () => {
  const nodes = new Map(
    [
      node("aocs", "group", null, ["RW", "STT"]),
      node("RW", "group", "aocs", ["RW_X", "RW_Y"]),
      node("RW_X", "part", "RW"),
      node("RW_Y", "part", "RW"),
      node("STT", "part", "aocs"),
      node("outer", "group", "aocs", ["RW"]),
    ].map((n) => [n.id, n]),
  );

  it("入れ子のグループは中の部品を返し、系そのものは解けない", () => {
    expect(ungroupParts(nodes.get("RW") as TreeNode, nodes)).toEqual(["RW_X", "RW_Y"]);
    expect(ungroupParts(nodes.get("aocs") as TreeNode, nodes)).toBeNull();
  });

  it("入れ子のグループを含むグループは、先に中を解く", () => {
    expect(ungroupParts(nodes.get("outer") as TreeNode, nodes)).toBeNull();
  });
});
