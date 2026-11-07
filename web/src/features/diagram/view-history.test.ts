import { describe, expect, it } from "vitest";
import type { Op, ViewLayout } from "@/api/types";
import { inverseOf, type ViewOp } from "./view-history";

/** テスト用: 図の配置の Op を配置に当てる（fune-ops の当て方と同じ意味） */
function apply(view: ViewLayout, ops: Op[]): ViewLayout {
  const next: ViewLayout = { ...view, nodes: { ...view.nodes }, edge: { ...view.edge } };
  for (const op of ops) {
    if (op.op === "set_view_layout") {
      for (const n of op.nodes) next.nodes[n.id] = { x: n.x, y: n.y, w: n.w, h: n.h };
      for (const id of op.remove) delete next.nodes[id];
    } else if (op.op === "set_view_edge") next.edge[op.key] = op.edge;
    else if (op.op === "unset_view_edge") delete next.edge[op.key];
    else if (op.op === "reset_view") {
      next.nodes = {};
      next.edge = {};
    }
  }
  return next;
}

const before: ViewLayout = {
  id: "electrical",
  nodes: { POWER: { x: 10, y: 20, w: 400, h: 200 }, PCDU: { x: 30, y: 40, w: null, h: null } },
  edge: {
    "P5V:PCDU.OUT2>CAM.PWR": {
      points: [
        [0, 0],
        [0, 50],
        [80, 50],
      ],
      source_side: "bottom",
    },
  },
};

describe("配置の編集を元に戻す Op（view-history.ts）", () => {
  const cases: [string, ViewOp[]][] = [
    [
      "前に位置があったノードと無かったノードを動かす",
      [
        {
          op: "set_view_layout",
          view: "electrical",
          nodes: [
            { id: "PCDU", x: 99, y: 98, w: null, h: null },
            { id: "CAM", x: 5, y: 6, w: null, h: null },
          ],
          remove: [],
        },
      ],
    ],
    [
      "位置を消す（自動配置に戻す）",
      [{ op: "set_view_layout", view: "electrical", nodes: [], remove: ["POWER", "PCDU"] }],
    ],
    [
      "線の経路を書き換える・新しく書く",
      [
        {
          op: "set_view_edge",
          view: "electrical",
          key: "P5V:PCDU.OUT2>CAM.PWR",
          edge: {
            points: [
              [1, 1],
              [1, 9],
            ],
          },
        },
        {
          op: "set_view_edge",
          view: "electrical",
          key: "CAN:OBC.CAN1>RW_X.DATA",
          edge: {
            points: [
              [2, 2],
              [9, 2],
            ],
          },
        },
      ],
    ],
    [
      "線の経路を消す",
      [{ op: "unset_view_edge", view: "electrical", key: "P5V:PCDU.OUT2>CAM.PWR" }],
    ],
    ["全体をリセットする", [{ op: "reset_view", view: "electrical" }]],
  ];
  for (const [name, ops] of cases)
    it(`${name}: 当ててから戻す Op を当てると元の配置になる`, () => {
      const after = apply(before, ops);
      expect(after).not.toEqual(before);
      expect(apply(after, inverseOf(before, ops))).toEqual(before);
    });

  it("前に位置が無かったノードは、戻すと位置を消す（自動配置の位置に戻る）", () => {
    const ops: ViewOp[] = [
      {
        op: "set_view_layout",
        view: "electrical",
        nodes: [{ id: "CAM", x: 5, y: 6, w: null, h: null }],
        remove: [],
      },
    ];
    expect(inverseOf(before, ops)).toEqual([
      { op: "set_view_layout", view: "electrical", nodes: [], remove: ["CAM"] },
    ]);
  });
});
