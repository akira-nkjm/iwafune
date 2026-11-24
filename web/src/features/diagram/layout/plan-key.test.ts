import { describe, expect, it } from "vitest";
import { diagramDemo } from "@/test/diagram-model";
import type { DiagramModel } from "../model";
import { keyOf, LAYOUT_VERSION, layoutInput, planKey } from "./plan-key";

const clone = (m: DiagramModel): DiagramModel => structuredClone(m);

describe("配置の結果の鍵", () => {
  it("同じ入力なら同じ鍵（作り方やキーの順に依らない）", async () => {
    const reordered: DiagramModel = {
      legend: [],
      edges: diagramDemo.edges.map(
        (e) => Object.fromEntries(Object.entries(e).reverse()) as typeof e,
      ),
      junctions: diagramDemo.junctions,
      cards: diagramDemo.cards,
      containers: diagramDemo.containers,
    };
    const base = await planKey({ ...diagramDemo, legend: [] });
    expect(await planKey(clone({ ...diagramDemo, legend: [] }))).toBe(base);
    expect(await planKey(reordered)).toBe(base);
  });

  it("配置が読む欄が変わったら別の鍵（カード・枠・線・ラベル・親）", async () => {
    const base = await planKey(diagramDemo);
    const at = <T>(list: T[]): T => {
      const x = list[0];
      if (!x) throw new Error("empty");
      return x;
    };
    const edits: ((m: DiagramModel) => void)[] = [
      (m) => {
        at(m.cards).label += "x";
      },
      (m) => {
        at(m.cards).ports.pop();
      },
      (m) => {
        at(m.containers).label += "x";
      },
      (m) => {
        at(m.containers).parent = "group:other";
      },
      (m) => {
        m.edges.pop();
      },
      (m) => {
        at(m.edges).label = "changed";
      },
      (m) => {
        m.cards.reverse();
      },
    ];
    for (const edit of edits) {
      const m = clone(diagramDemo);
      edit(m);
      expect(await planKey(m)).not.toBe(base);
    }
  });

  it("配置が読まない凡例は鍵に入れない", async () => {
    const m = clone(diagramDemo);
    m.legend = [];
    expect(layoutInput(m)).toBe(layoutInput(diagramDemo));
    expect(await planKey(m)).toBe(await planKey(diagramDemo));
  });

  it("アルゴリズムの版が変わったら別の鍵", async () => {
    expect(await keyOf(diagramDemo, LAYOUT_VERSION)).toBe(await planKey(diagramDemo));
    expect(await keyOf(diagramDemo, LAYOUT_VERSION + 1)).not.toBe(await planKey(diagramDemo));
  });
});
