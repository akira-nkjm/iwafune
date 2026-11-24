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
  /** dry_run の結果（CLI のプレビュー）。revision が変われば別のキー */
  dryRun: (revision: string, opsJson: string) => ["dry_run", revision, opsJson] as const,
  /** 打った文字列から作った Op（`POST /ops/from_text`。書き込まない）。revision が変われば別のキー */
  textOp: (revision: string, textJson: string) => ["text_op", revision, textJson] as const,
  /** 名詞のコマンドに打った引数から作った Op の列（`POST /ops/from_command`。書き込まない） */
  commandOps: (revision: string, commandJson: string) =>
    ["command_ops", revision, commandJson] as const,
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
