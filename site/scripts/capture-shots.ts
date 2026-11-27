/**
 * docs/ の画面写真の目印（`<!-- screenshot: <route> [as <name>] — <description> -->`）から、実際の画面の写真を撮る。
 *
 *   pnpm -C site shots -- --fune <path of the fune binary> [--only <id>[,<id>]] [--lang en|ja] [--browser <chromium path>]
 *
 * 環境変数 FUNE_BIN で `--fune` を、CHROMIUM_PATH で `--browser` を渡してもよい（`--browser` を省くと Playwright が入れた Chromium を使う。
 * 入れるには `pnpm -C site exec playwright-core install chromium`）。
 * 自分で全部を用意する: デモを一時ディレクトリにコピーし、シナリオ（scripts/shots/scenarios.ts）のコマンドを実行して、
 * `fune gui --no-open` を空いているポートで起動し、1440×900・2 倍の解像度・ライトテーマで英語と日本語の両方を撮る。
 * 写真は `site/src/assets/shots/<id>.png`（日本語は `<id>.ja.png`）。サーバーとブラウザーは、失敗したときも必ず止める。
 */
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { type Browser, chromium, type Page } from "playwright-core";
import { findMarkers, shotFileName, shotId } from "../src/lib/shots.ts";
import { docsDir, repoRoot, SHOTS_DIR } from "../src/lib/site.ts";
import {
  type Gui,
  type ProjectSpec,
  prepareProject,
  projectKey,
  startGui,
} from "./shots/project.ts";
import { DEMO, SCENARIOS, type Scenario } from "./shots/scenarios.ts";
import { settle, trackApi } from "./shots/wait.ts";

/** GUI の言語の保存先（localStorage のキー）。 */
const LANG_KEY = "iwafune.lang";
const VIEWPORT = { width: 1440, height: 900 };

type Lang = "en" | "ja";

/** 撮る写真 1 枚。 */
export interface Shot {
  id: string;
  route: string;
  name?: string;
  description: string;
}

/** コマンドラインの引数。 */
export interface Args {
  fune: string;
  browser?: string;
  only: string[];
  langs: Lang[];
}

/** 引数を読む。 */
export function parseArgs(argv: string[], env: NodeJS.ProcessEnv = process.env): Args {
  const get = (name: string) => {
    const i = argv.indexOf(`--${name}`);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const fune = get("fune") ?? env.FUNE_BIN;
  if (!fune) throw new Error("give the fune binary with --fune <path> (or FUNE_BIN)");
  const lang = get("lang");
  if (lang && lang !== "en" && lang !== "ja") throw new Error("--lang must be en or ja");
  const browser = get("browser") ?? env.CHROMIUM_PATH;
  return {
    fune: path.resolve(fune),
    ...(browser ? { browser } : {}),
    only: (get("only") ?? "").split(",").filter(Boolean),
    langs: lang ? [lang as Lang] : ["en", "ja"],
  };
}

/** docs/ の下の Markdown（英語・日本語）にある目印から、撮る写真を（識別子ごとに 1 つ）集める。 */
export function collectShots(docs: string): Shot[] {
  const out = new Map<string, Shot>();
  const files = fs.readdirSync(docs, { recursive: true, encoding: "utf-8" }) as string[];
  for (const f of files.filter((n) => n.endsWith(".md") && !n.startsWith("ja")).sort()) {
    for (const m of findMarkers(fs.readFileSync(path.join(docs, f), "utf-8"))) {
      const id = shotId(m.route, m.name);
      if (!out.has(id)) {
        out.set(id, {
          id,
          route: m.route,
          ...(m.name ? { name: m.name } : {}),
          description: m.description,
        });
      }
    }
  }
  return [...out.values()];
}

/** 写真を撮る前に、目印とシナリオの食い違いを見つける（名前付きなのにシナリオが無い、目印の無いシナリオ）。 */
export function checkScenarios(shots: Shot[], scenarios: Record<string, Scenario>): string[] {
  const ids = new Set(shots.map((s) => s.id));
  const problems: string[] = [];
  for (const s of shots) {
    if (s.name && !scenarios[s.id]) problems.push(`named shot ${s.id} has no scenario`);
  }
  for (const id of Object.keys(scenarios)) {
    if (!ids.has(id)) problems.push(`scenario ${id} has no marker in docs/`);
  }
  return problems;
}

/** 画面に出ている手元のパス（一時ディレクトリ）を、どの環境でも同じ見え方の仮のパスに置き換える（写真に手元の場所を残さない）。 */
async function hideLocalPath(page: Page, dir: string): Promise<void> {
  const shown = `~/${path.basename(dir)}`;
  await page.evaluate(
    ([from, to]) => {
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
      for (let n = walker.nextNode(); n; n = walker.nextNode()) {
        if (n.nodeValue?.includes(from as string)) {
          n.nodeValue = n.nodeValue.split(from as string).join(to as string);
        }
      }
    },
    [dir, shown],
  );
}

/** 1 つのプロジェクトの写真を、言語ごとに撮る。 */
async function captureGroup(
  browser: Browser,
  gui: Gui,
  dir: string,
  shots: Shot[],
  langs: Lang[],
): Promise<void> {
  for (const lang of langs) {
    const context = await browser.newContext({
      viewport: VIEWPORT,
      deviceScaleFactor: 2,
      colorScheme: "light",
      locale: lang === "ja" ? "ja-JP" : "en-US",
    });
    await context.addInitScript(
      ([key, value]) => {
        try {
          localStorage.setItem(key as string, value as string);
        } catch {
          // 保存先が使えなくても撮れる（言語が既定になるだけ）。
        }
      },
      [LANG_KEY, lang],
    );
    try {
      for (const shot of shots) {
        const label = `"${shot.id}" (${lang})`;
        const scenario = SCENARIOS[shot.id];
        const page = await context.newPage();
        try {
          if (scenario?.height)
            await page.setViewportSize({ ...VIEWPORT, height: scenario.height });
          const api = trackApi(page);
          await page.goto(`${gui.base}${shot.route}`);
          await settle(page, api, label);
          const ctx = { page, label, settle: () => settle(page, api, label) };
          await scenario?.steps?.(ctx);
          await hideLocalPath(page, dir);
          const file = path.join(SHOTS_DIR, shotFileName(shot.route, lang, shot.name));
          const target = scenario?.clip?.(page);
          const png = target ? await target.screenshot() : await page.screenshot();
          fs.writeFileSync(file, png);
          console.log(`ok  ${path.relative(process.cwd(), file)}  (${shot.description})`);
        } catch (e) {
          const msg = e instanceof Error ? e.message : String(e);
          throw new Error(msg.startsWith("shot ") ? msg : `shot ${label}: ${msg}`);
        } finally {
          await page.close();
        }
      }
    } finally {
      await context.close();
    }
  }
}

async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2).filter((a) => a !== "--"));
  const all = collectShots(docsDir());
  const problems = checkScenarios(all, SCENARIOS);
  if (problems.length > 0) throw new Error(problems.join("\n"));
  const shots = all.filter((s) => args.only.length === 0 || args.only.includes(s.id));
  if (shots.length === 0) {
    console.log("no screenshot markers found");
    return;
  }
  // プロジェクトごとにまとめる（同じプロジェクトの写真は 1 つのサーバーで撮る）。
  const groups = new Map<string, { spec: ProjectSpec; shots: Shot[] }>();
  for (const shot of shots) {
    const spec = SCENARIOS[shot.id]?.project ?? DEMO;
    const key = projectKey(spec);
    const group = groups.get(key) ?? { spec, shots: [] };
    group.shots.push(shot);
    groups.set(key, group);
  }
  fs.mkdirSync(SHOTS_DIR, { recursive: true });
  const work = fs.mkdtempSync(path.join(os.tmpdir(), "iwafune-shots-"));
  let browser: Browser | undefined;
  let gui: Gui | undefined;
  try {
    browser = await chromium.launch({
      ...(args.browser ? { executablePath: args.browser } : {}),
      // GPU の無い環境でも 3D の画面（WebGL）を描けるようにする。
      args: ["--enable-unsafe-swiftshader", "--use-angle=swiftshader", "--hide-scrollbars"],
    });
    let index = 0;
    for (const group of groups.values()) {
      const dir = prepareProject(group.spec, index++, { fune: args.fune, work, repo: repoRoot() });
      gui = await startGui(args.fune, dir);
      try {
        await captureGroup(browser, gui, dir, group.shots, args.langs);
      } finally {
        await gui.stop();
        gui = undefined;
      }
    }
  } finally {
    await gui?.stop();
    await browser?.close();
    fs.rmSync(work, { recursive: true, force: true });
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((e: unknown) => {
    console.error(e instanceof Error ? e.message : e);
    process.exit(1);
  });
}
