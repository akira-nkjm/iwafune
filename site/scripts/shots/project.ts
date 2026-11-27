/**
 * 写真を撮る対象のプロジェクトの用意と、GUI のサーバーの起動・停止。
 * デモを一時ディレクトリにコピーして使うので、リポジトリの fixtures は書き換えない。
 */
import { type ChildProcess, execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { consoleCommands, tokenize } from "./commands.ts";

/** 写真の対象のプロジェクト。 */
export type ProjectSpec =
  /** fixtures/demo-6u のコピー。`cli` のコマンドを順に実行してから開く。 */
  | { kind: "demo"; cli?: string[] }
  /** `fune init` したての空のプロジェクト。`cli` のコマンドを順に実行してから開く。 */
  | { kind: "init"; cli?: string[] }
  /** 文書 `docs/guide/first-satellite.md` が載せているコマンドを、`stopAt` の目印の手前まで順に実行したプロジェクト。 */
  | { kind: "tutorial"; stopAt: string };

/** 使ってはいけないポート（ほかの用途と取り違えないため）。 */
export const FORBIDDEN_PORTS = new Set([3000, 5173, 4321, 3471]);

/** プロジェクトの識別キー（同じキーのシナリオは 1 つのサーバーを共有する）。 */
export function projectKey(spec: ProjectSpec): string {
  return JSON.stringify(spec);
}

interface Env {
  fune: string;
  /** 作業用のディレクトリ（一時ディレクトリ）。 */
  work: string;
  repo: string;
}

/** コマンド 1 行を実行する。`fune` と `git` と `cd` だけを受け付け、失敗したら出力つきで誤りにする。 */
function runLine(line: string, cwd: string, env: Env): string {
  const [cmd, ...args] = tokenize(line);
  if (cmd === "cd") return path.resolve(cwd, args[0] ?? ".");
  const bin = cmd === "fune" ? env.fune : cmd === "git" ? "git" : null;
  if (!bin) throw new Error(`unsupported command in a shot scenario: ${line}`);
  try {
    execFileSync(bin, args, { cwd, stdio: "pipe", env: { ...process.env, NO_COLOR: "1" } });
  } catch (e) {
    const err = e as { stderr?: Buffer; stdout?: Buffer; message: string };
    throw new Error(
      `command failed: ${line}\n${err.stderr?.toString() ?? ""}${err.stdout?.toString() ?? err.message}`,
    );
  }
  return cwd;
}

/** プロジェクトを用意して、そのディレクトリを返す。 */
export function prepareProject(spec: ProjectSpec, index: number, env: Env): string {
  const root = path.join(env.work, `project-${index}`);
  fs.mkdirSync(root, { recursive: true });
  if (spec.kind === "demo") {
    const dir = path.join(root, "iwafune-6u-demo");
    fs.cpSync(path.join(env.repo, "fixtures", "demo-6u"), dir, { recursive: true });
    for (const line of spec.cli ?? []) runLine(line, dir, env);
    return dir;
  }
  if (spec.kind === "init") {
    runLine("fune init sat", root, env);
    const dir = path.join(root, "sat");
    for (const line of spec.cli ?? []) runLine(line, dir, env);
    return dir;
  }
  const page = fs.readFileSync(path.join(env.repo, "docs", "guide", "first-satellite.md"), "utf-8");
  let cwd = root;
  for (const line of consoleCommands(page, spec.stopAt)) cwd = runLine(line, cwd, env);
  return cwd;
}

/** 空いているポートを 1 つ得る（使ってはいけないポートは避ける）。 */
async function freePort(): Promise<number> {
  for (let i = 0; i < 20; i++) {
    const port = await new Promise<number>((resolve, reject) => {
      const s = net.createServer();
      s.once("error", reject);
      s.listen(0, "127.0.0.1", () => {
        const p = (s.address() as net.AddressInfo).port;
        s.close(() => resolve(p));
      });
    });
    if (!FORBIDDEN_PORTS.has(port)) return port;
  }
  throw new Error("could not find a free port");
}

/** 動いている GUI のサーバー。 */
export interface Gui {
  base: string;
  stop(): Promise<void>;
}

/** `fune -C <dir> gui --no-open --port <port>` を起動して、応答するまで待つ。 */
export async function startGui(fune: string, dir: string): Promise<Gui> {
  const port = await freePort();
  const child: ChildProcess = spawn(fune, ["-C", dir, "gui", "--no-open", "--port", String(port)], {
    stdio: ["ignore", "ignore", "pipe"],
  });
  let stderr = "";
  child.stderr?.on("data", (d: Buffer) => {
    stderr += d.toString();
  });
  const exited = new Promise<never>((_, reject) => {
    child.once("exit", (code) => reject(new Error(`fune gui exited (code ${code}): ${stderr}`)));
  });
  const stop = async () => {
    if (child.exitCode !== null || child.signalCode !== null) return;
    const done = new Promise<void>((resolve) => child.once("exit", () => resolve()));
    child.kill("SIGTERM");
    await Promise.race([done, new Promise((r) => setTimeout(r, 5000))]);
    if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
  };
  const base = `http://127.0.0.1:${port}`;
  const ready = (async () => {
    for (let i = 0; i < 150; i++) {
      try {
        if ((await fetch(`${base}/`)).ok) return;
      } catch {
        // まだ起動中。
      }
      await new Promise((r) => setTimeout(r, 200));
    }
    throw new Error(`fune gui did not answer on port ${port} within 30 s`);
  })();
  try {
    await Promise.race([ready, exited]);
  } catch (e) {
    await stop();
    throw e;
  }
  // 起動したあとの終了は、停止のときだけ。
  exited.catch(() => {});
  return { base, stop };
}
