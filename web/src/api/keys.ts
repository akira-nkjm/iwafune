/**
 * TanStack Query のキーはここだけで作る。編集（POST /ops）の後に無効化する範囲もここで決める。
 */
export const keys = {
  project: ["project"] as const,
  design: ["design"] as const,
  /** 1 部品。`opsJson` は保存していない編集（下書き）を当てたもの（空なら保存済みの設計） */
  part: (id: string, opsJson = "") =>
    opsJson ? (["design", "part", id, opsJson] as const) : (["design", "part", id] as const),
  defs: ["design", "defs"] as const,
  systemModes: (system: string, opsJson: string) =>
    ["design", "system_modes", system, opsJson] as const,
  diagnostics: ["diagnostics"] as const,
  units: (dimension?: string | null) => ["units", dimension ?? null] as const,
  parse: (input: string, dimension: string | null) => ["units", "parse", input, dimension] as const,
  analyses: ["analyses"] as const,
  /** すべての解析の結果（下の `analysis` の先頭。設計の編集と天体の暦の読み込みで無効化する） */
  analysisResults: ["analysis"] as const,
  /** 電源ページも解析の編集・暦の取得で無効化する。 */
  powerPage: (kase: string | null) => ["analysis", "power_page", kase] as const,
  /** 解析の結果。`opsJson` は保存していない編集（下書き）を当てて計算したもの（空なら保存済みの設計） */
  analysis: (id: string, kase: string | null, opsJson = "") =>
    opsJson ? (["analysis", id, kase, opsJson] as const) : (["analysis", id, kase] as const),
  /** 外部データ（DE440）の状態（`GET /data/status`）。設計の編集では変わらない */
  dataStatus: ["data", "status"] as const,
  /** バジェット。`opsJson` の意味は analysis と同じ */
  budget: (kind: string, opsJson = "") =>
    opsJson ? (["budget", kind, opsJson] as const) : (["budget", kind] as const),
  /** dry_run の結果（CLI のプレビュー）。revision が変われば別のキー */
  dryRun: (revision: string, opsJson: string) => ["dry_run", revision, opsJson] as const,
  /** 打った文字列から作った Op（`POST /ops/from_text`。書き込まない）。revision が変われば別のキー */
  textOp: (revision: string, textJson: string) => ["text_op", revision, textJson] as const,
  /** 名詞のコマンドに打った引数から作った Op の列（`POST /ops/from_command`。書き込まない） */
  commandOps: (revision: string, commandJson: string) =>
    ["command_ops", revision, commandJson] as const,
};

/** どの Op の後でも無効化するキーの先頭（設計・診断・解析・バジェットはすべて設計から導出される） */
export const invalidatedByOps: readonly (readonly string[])[] = [
  keys.project,
  keys.design,
  keys.diagnostics,
  keys.analysisResults,
  ["budget"],
];
