import { describe, expect, it } from "vitest";
import { readFlipped, writeFlipped } from "./overview-fold-state";

function memory() {
  const data = new Map<string, string>();
  return {
    data,
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, v),
  };
}

describe("一覧の枠の状態の保存", () => {
  it("書いた ID を、同じプロジェクトで読み戻す", () => {
    const s = memory();
    writeFlipped("demo", new Set(["SAP", "WING"]), () => true, s);
    expect([...readFlipped("demo", s)].sort()).toEqual(["SAP", "WING"]);
  });

  it("プロジェクトごとに分ける", () => {
    const s = memory();
    writeFlipped("a", new Set(["SAP"]), () => true, s);
    expect(readFlipped("b", s).size).toBe(0);
  });

  it("もう無い枠の ID は保存しない", () => {
    const s = memory();
    writeFlipped("demo", new Set(["SAP", "GONE"]), (id) => id !== "GONE", s);
    expect([...readFlipped("demo", s)]).toEqual(["SAP"]);
  });

  it("保存が無い・壊れている・保存先が使えないときは空", () => {
    const s = memory();
    expect(readFlipped("demo", s).size).toBe(0);
    s.data.set("iwafune.components.flipped.demo", "{not json");
    expect(readFlipped("demo", s).size).toBe(0);
    expect(readFlipped("demo", null).size).toBe(0);
    const broken = {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => {
        throw new Error("denied");
      },
    };
    expect(readFlipped("demo", broken).size).toBe(0);
    expect(() => writeFlipped("demo", new Set(["SAP"]), () => true, broken)).not.toThrow();
  });
});
