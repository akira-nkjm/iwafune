/**
 * TanStack Query のキーはここだけで作る。編集（POST /ops）の後に無効化する範囲もここで決める。
 */
export const keys = {
  project: ["project"] as const,
  design: ["design"] as const,
  part: (id: string) => ["design", "part", id] as const,
  defs: ["design", "defs"] as const,
  diagnostics: ["diagnostics"] as const,
  units: (dimension?: string | null) => ["units", dimension ?? null] as const,
  parse: (input: string, dimension: string | null) => ["units", "parse", input, dimension] as const,
  analyses: ["analyses"] as const,
  /** 解析の結果。`opsJson` は保存していない編集（下書き）を当てて計算したもの（空なら保存済みの設計） */
  analysis: (id: string, kase: string | null, opsJson = "") =>
    opsJson ? (["analysis", id, kase, opsJson] as const) : (["analysis", id, kase] as const),
  /** バジェット。`opsJson` の意味は analysis と同じ */
  budget: (kind: string, opsJson = "") =>
    opsJson ? (["budget", kind, opsJson] as const) : (["budget", kind] as const),
  /** dry_run の結果（CLI のプレビュー）。revision が変われば別のキー */
  dryRun: (revision: string, opsJson: string) => ["dry_run", revision, opsJson] as const,
};

/** どの Op の後でも無効化するキーの先頭（設計・診断・解析・バジェットはすべて設計から導出される） */
export const invalidatedByOps: readonly (readonly string[])[] = [
  keys.project,
  keys.design,
  keys.diagnostics,
  ["analysis"],
  ["budget"],
];
