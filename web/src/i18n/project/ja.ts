/** プロジェクト（診断・設定）の文言（日本語） */
export const projectJa = {
  project: {
    eyebrow: "プロジェクト",
    diagnostics: "診断",
    diagSubtitle: (e: number, w: number, i: number) => `エラー ${e} · 警告 ${w} · 情報 ${i}`,
    filterBySeverity: "重さで絞る",
    cannotLoadDiagnostics: (msg: string) => `診断を読めません: ${msg}`,
    noDiagnostics: "該当する診断はありません",
    settings: "設定",
    data: {
      title: "外部データ",
      /** 解析が使う天体の暦（API の `DataStatus.ephemeris`） */
      ephemeris: { de440: "天体の暦: DE440", analytic: "天体の暦: 解析式" },
      installed: "読み込み済み",
      cached: "キャッシュ済み",
      missing: "未取得",
      fetching: "取得中…",
      fetch: "取得",
      /** 取得した日時（毎日更新される宇宙天気の表など。API の `DataItem.fetched_at`） */
      fetchedAt: (at: string) => `取得 ${at}`,
    },
    projectPanel: "プロジェクト",
    name: "名前",
    root: "場所",
    branch: "ブランチ",
    notGit: "（git ではない）",
    revision: "リビジョン",
    schema: "スキーマの版",
    groups: {
      conditions: { title: "前提条件", caption: "conditions/ — 軌道・地上局・期間" },
      analysis: { title: "解析", caption: "analysis/ — 各系の解析の設定" },
      budget: { title: "バジェット", caption: "budget/ — 配分・マージン方針" },
    },
    noSettings: "設定がありません",
    key: "キー",
    value: "値",
    file: "ファイル",
  },
};
