import type { DesignView, Op } from "./types";

/**
 * TanStack Query のキーはここだけで作る。編集（POST /ops）の後に無効化する範囲もここで決める。
 */
export const keys = {
  project: ["project"] as const,
  design: ["design"] as const,
  parts: (configuration: string | null) => ["design", "parts", configuration] as const,
  /** 1 部品。`opsJson` は保存していない編集（下書き）を当てたもの（空なら保存済みの設計） */
  part: (id: string, opsJson = "", configuration: string | null = null) =>
    configuration
      ? (["design", "part", id, opsJson, configuration] as const)
      : opsJson
        ? (["design", "part", id, opsJson] as const)
        : (["design", "part", id] as const),
  defs: ["design", "defs"] as const,
  /** 系別のモード表示（部品別の値の表。解析は回さない）。`summary` が既定（all）以外のときは足もとの行の範囲を足す */
  systemModes: (system: string, opsJson: string, summary = "all") =>
    summary === "all"
      ? (["design", "system_modes", system, opsJson] as const)
      : (["design", "system_modes", system, opsJson, summary] as const),
  /** 解析も回した系別のモード表示（電源のモードタブ・時系列タブ）。解析の再計算・暦の取得でも無効化する */
  analyzedModes: (system: string, kase: string | null, opsJson: string) =>
    ["analysis", "system_modes", system, kase, opsJson] as const,
  /** 解析も回した系別のモード表示すべて（「再計算」で無効化する先頭） */
  analyzedModesAll: ["analysis", "system_modes"] as const,
  /** 熱の結合・表面の表。`opsJson` は保存していない編集（下書き）を当てたもの（空なら保存済みの設計）。解析の結果と同じく編集で無効化する */
  thermalTables: (kase: string | null, opsJson: string) =>
    ["analysis", "thermal_tables", kase, opsJson] as const,
  diagnostics: ["diagnostics"] as const,
  /** 編集の履歴（`GET /history`。取り消し・やり直しができるか）。編集・取り消し・やり直しのたびに変わる */
  history: ["history"] as const,
  units: (dimension?: string | null) => ["units", dimension ?? null] as const,
  parse: (input: string, dimension: string | null) => ["units", "parse", input, dimension] as const,
  analyses: ["analyses"] as const,
  /** すべての解析の結果（下の `analysis` の先頭。設計の編集と天体の暦の読み込みで無効化する） */
  analysisResults: ["analysis"] as const,
  /** 解析の結果。`opsJson` は保存していない編集（下書き）を当てて計算したもの（空なら保存済みの設計）。`configuration` は明示で選んだ形態（無ければ設計の選択に従う） */
  analysis: (id: string, kase: string | null, opsJson = "", configuration: string | null = null) =>
    configuration
      ? (["analysis", id, kase, opsJson, configuration] as const)
      : opsJson
        ? (["analysis", id, kase, opsJson] as const)
        : (["analysis", id, kase] as const),
  /**
   * 解析の重い任意の出力（`detail: true` で取り直したもの。メッシュ上の場など）。結果と同じ先頭なので、同じ無効化の範囲に入る。
   * 結果が使ったケースと形態を指定して取るので、キーにも入れる
   */
  analysisDetail: (id: string, kase: string | null, configuration: string | null = null) =>
    ["analysis", id, kase, "detail", configuration] as const,
  /** 外部データ（DE440）の状態（`GET /data/status`）。設計の編集では変わらない */
  dataStatus: ["data", "status"] as const,
  /** バジェット。`opsJson` の意味は analysis と同じ */
  budget: (kind: string, opsJson = "") =>
    opsJson ? (["budget", kind, opsJson] as const) : (["budget", kind] as const),
  /** 要求適合表（`GET /requirements/compliance`。設計・解析・バジェットから導出される） */
  compliance: ["compliance"] as const,
  /** Op を確定する mutation のキー（`useApplyOps` など）。確定の最中かを `useIsMutating` で見るのに使う */
  applyOps: ["apply_ops"] as const,
  /** dry_run の結果（CLI のプレビュー）。revision が変われば別のキー */
  dryRun: (revision: string, opsJson: string) => ["dry_run", revision, opsJson] as const,
  /** 打った文字列から作った Op（`POST /ops/from_text`。書き込まない）。revision が変われば別のキー */
  textOp: (revision: string, textJson: string) => ["text_op", revision, textJson] as const,
  /** 名詞のコマンドに打った引数から作った Op の列（`POST /ops/from_command`。書き込まない） */
  commandOps: (revision: string, commandJson: string) =>
    ["command_ops", revision, commandJson] as const,
  /** 最悪の日から作るケースの Op の列（`POST /ops/cases_from_geometry`。書き込まない）。revision が変われば別のキー */
  geometryCaseOps: (revision: string | null, json: string) =>
    ["geometry-case-ops", revision, json] as const,
  /** 季節解析から作った新しいケースの Op（`POST /ops/case_from_eclipse`。書き込まない）。revision が変われば別のキー */
  eclipseCaseOp: (revision: string | null, json: string) =>
    ["eclipse-case-op", revision, json] as const,
};

/** どの Op の後でも無効化するキーの先頭（設計・診断・解析・バジェットはすべて設計から導出される） */
export const invalidatedByOps: readonly (readonly string[])[] = [
  keys.project,
  keys.design,
  keys.diagnostics,
  keys.history,
  keys.analysisResults,
  ["budget"],
  keys.compliance,
];

/**
 * 編集で消える・名前が変わる対象（変わる前の ID）。その ID をキーに持つ問い合わせは、編集のあとは 404 になる。
 * 持っているのは、キーに ID が入る種類だけ（部品・形態・解析ケース）。ほかの対象（モード・フェーズ・マヌーバー・要求・
 * 地上局・カタログ品・グループ）は問い合わせのキーに入らない
 */
export interface Retired {
  parts: ReadonlySet<string>;
  configurations: ReadonlySet<string>;
  cases: ReadonlySet<string>;
}

/** 送った Op のうち、消す・名前を変えるものの対象（変わる前の ID） */
export function retiredBy(ops: readonly Op[]): Retired {
  const parts = new Set<string>();
  const configurations = new Set<string>();
  const cases = new Set<string>();
  for (const o of ops) {
    if (o.op === "remove_part" || o.op === "rename_part") parts.add(o.id);
    else if (o.op === "remove_configuration" || o.op === "rename_configuration")
      configurations.add(o.id);
    else if (o.op === "remove_case" || o.op === "rename_case") cases.add(o.id);
  }
  return { parts, configurations, cases };
}

/**
 * 変わる前の設計にあって、変わった後の設計に無い対象（変わる前の ID）。取り消し・やり直しは、
 * 何を消した・名前を変えたかを返さないので、前後の設計を比べて求める（`retiredBy` と同じ範囲の種類だけ）
 */
export function retiredBetween(before: DesignView, after: DesignView): Retired {
  const gone = (was: readonly { id: string }[], now: readonly { id: string }[]) => {
    const live = new Set(now.map((x) => x.id));
    return new Set(was.filter((x) => !live.has(x.id)).map((x) => x.id));
  };
  return {
    parts: gone(before.parts, after.parts),
    configurations: gone(before.configurations, after.configurations),
    cases: gone(before.cases, after.cases),
  };
}

const isEmpty = (r: Retired) => r.parts.size + r.configurations.size + r.cases.size === 0;
const has = (set: ReadonlySet<string>, v: unknown) => typeof v === "string" && set.has(v);

/**
 * このキーの問い合わせは、消えた・名前が変わった対象を指しているか。上の `keys` の形に合わせて読む
 * （並びを変えたら、ここと keys.test.ts を合わせる）:
 * - `parts(configuration)` = `design, parts, 形態`
 * - `part(id, ops, configuration)` = `design, part, ID, ops, 形態`（形態は 5 番目のときだけ）
 * - `analysis(id, case, ops, configuration)` と `analysisDetail` = `analysis, ID, ケース, ops|detail, 形態`（形態は 5 番目のときだけ）
 * - `analyzedModes(system, case, ops)` = `analysis, system_modes, 系, ケース, ops`
 * - `thermalTables(case, ops)` = `analysis, thermal_tables, ケース, ops`
 */
export function refersToRetired(key: readonly unknown[], r: Retired): boolean {
  if (isEmpty(r)) return false;
  if (key[0] === "design") {
    if (key[1] === "parts") return has(r.configurations, key[2]);
    if (key[1] === "part")
      return has(r.parts, key[2]) || (key.length === 5 && has(r.configurations, key[4]));
    return false;
  }
  if (key[0] !== "analysis") return false;
  if (key[1] === "system_modes") return has(r.cases, key[3]);
  if (key[1] === "thermal_tables") return has(r.cases, key[2]);
  return has(r.cases, key[2]) || (key.length === 5 && has(r.configurations, key[4]));
}
