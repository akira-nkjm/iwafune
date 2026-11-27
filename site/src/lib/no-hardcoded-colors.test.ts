/**
 * 色の値は styles/tokens.css だけに書く。ほかの CSS・コンポーネント・スクリプトに 16 進の色や rgb() が無いことを検査する
 * （web/ の xtask の検査に相当するものを、サイトの中で完結させる）。
 */
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const root = path.resolve(import.meta.dirname, "../..");
const dirs = ["src", "scripts", "astro.config.ts"];
const SKIP = new Set(["tokens.css", "tokens.test.ts", "no-hardcoded-colors.test.ts"]);
const EXT = /\.(css|astro|ts|svg)$/;
const COLOR = /#[0-9a-fA-F]{3,8}\b|\brgba?\(|\bhsla?\(|\bcolor-mix\(/;

function files(p: string): string[] {
  const full = path.join(root, p);
  if (!fs.existsSync(full)) return [];
  if (fs.statSync(full).isFile()) return [p];
  return fs
    .readdirSync(full, { withFileTypes: true })
    .flatMap((e) => (e.name === "assets" ? [] : files(path.join(p, e.name))));
}

describe("色の正本", () => {
  it("tokens.css のほかに色の値が無い", () => {
    const bad: string[] = [];
    for (const f of dirs.flatMap(files)) {
      if (SKIP.has(path.basename(f)) || !EXT.test(f)) continue;
      const text = fs.readFileSync(path.join(root, f), "utf-8");
      text.split("\n").forEach((line, i) => {
        // `#top` のような見出しへのリンクは色ではない。色の 16 進は英数字だけで始まる 3〜8 桁。
        const m = COLOR.exec(line);
        if (m && !/href=|"#[a-z-]+"|#_top|#\d/.test(line))
          bad.push(`${f}:${i + 1}: ${line.trim()}`);
      });
    }
    expect(bad).toEqual([]);
  });
});
