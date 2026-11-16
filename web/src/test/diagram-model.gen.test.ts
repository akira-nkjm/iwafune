import { readFileSync, writeFileSync } from "node:fs";
import { it } from "vitest";
import type { DesignView } from "@/api/types";
import { buildModel } from "@/features/diagram/model";
import { toneHex } from "@/lib/tone";

/**
 * diagram-model.ts（デモの系統図の表示モデルの写し）を作り直す。表示モデル（features/diagram/model.ts）を変えたら実行する:
 *   curl -s http://127.0.0.1:3000/api/v1/design > /tmp/design.json   # fixtures/demo-6u を開いた fune gui
 *   DESIGN_JSON=/tmp/design.json pnpm -C web exec vitest run src/test/diagram-model.gen.test.ts
 * DESIGN_JSON が無ければ何もしない（ふだんのテストでは飛ばす）。
 */
const source = process.env.DESIGN_JSON;

it.skipIf(!source)("系統図の表示モデルの写しを作り直す", () => {
  const design = JSON.parse(readFileSync(source ?? "", "utf8")) as DesignView;
  const model = buildModel(design, { layers: new Set(["power", "signal"]), mergeSiblings: true });
  // 色は lib/tone.ts の名前で書く（色の直書きは tone.ts だけ）
  const names = new Map(Object.entries(toneHex).map(([k, v]) => [v.toLowerCase(), k]));
  const line = (row: unknown) =>
    JSON.stringify(row).replace(/"(#[0-9a-f]{6})"/gi, (_, hex: string) => {
      const name = names.get(hex.toLowerCase());
      if (!name) throw new Error(`tone.ts に無い色: ${hex}`);
      return `toneHex.${name}`;
    });
  const sections = (["containers", "cards", "junctions", "edges", "legend"] as const)
    .map((k) => `  ${k}: [\n${model[k].map((r) => `    ${line(r)},`).join("\n")}\n  ],`)
    .join("\n");
  const target = new URL("./diagram-model.ts", import.meta.url);
  const head = readFileSync(target, "utf8").split("export const diagramDemo")[0];
  writeFileSync(target, `${head}export const diagramDemo: DiagramModel = {\n${sections}\n};\n`);
});
