import { expect, it } from "vitest";
import { placeLabels } from "./labels";

it("ラベルは平行な線・カード・先に置いたラベルと重ならない場所に置く", () => {
  const routes = new Map([
    [
      "CAN",
      [
        { x: 0, y: 0 },
        { x: 300, y: 0 },
      ],
    ],
    [
      "LVDS",
      [
        { x: 0, y: 20 },
        { x: 300, y: 20 },
      ],
    ],
    [
      "SPW",
      [
        { x: 200, y: -100 },
        { x: 200, y: 100 },
      ],
    ],
  ]);
  const cards = [{ x: 0, y: -40, width: 80, height: 80 }];
  const labels = new Map([
    ["CAN", "CAN"],
    ["LVDS", "LVDS"],
    ["SPW", "SpaceWire"],
  ]);
  const positions = placeLabels(routes, labels, cards);
  expect(positions.size).toBe(3);
  const boxes = cards.slice();
  for (const [id, p] of positions) {
    const width = Math.max(28, (labels.get(id)?.length ?? 0) * 8 + 8);
    const r = { x: p.x - width / 2, y: p.y - 8, width, height: 16 };
    expect(
      boxes.some(
        (b) =>
          r.x < b.x + b.width &&
          r.x + r.width > b.x &&
          r.y < b.y + b.height &&
          r.y + r.height > b.y,
      ),
    ).toBe(false);
    for (const pts of routes.values()) {
      const a = pts[0];
      const b = pts[1];
      if (!a || !b) throw new Error("segment");
      expect(
        Math.min(a.x, b.x) < r.x + r.width &&
          Math.max(a.x, b.x) > r.x &&
          Math.min(a.y, b.y) < r.y + r.height &&
          Math.max(a.y, b.y) > r.y,
      ).toBe(false);
    }
    boxes.push(r);
  }
});
