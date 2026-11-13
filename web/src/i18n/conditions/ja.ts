/** 前提条件（軌道と地上局・解析ケース）の文言（日本語） */
export const conditionsJa = {
  conditions: {
    eyebrow: "前提条件",
    cases: "解析ケース",
    casesSubtitle: "cases/<ID>.toml（モードの時系列・不確かさの側・軌道・環境）",
    noCases: "解析ケースがありません（解析は既定のケースで動きます）",
    name: "名前",
    description: "説明",
    modes: "モード",
    file: "ファイル",
    openWith: "このケースで開く",
    orbit: "軌道と地上局",
    orbitSubtitle: "conditions/ の設定（軌道・地上局・期間）と、地上局の可視",
    conditionsSettings: "前提条件の設定",
    conditionsSettingsCaption: "キーが conditions. で始まる設定",
    noConditionsSettings: "conditions/ の設定がありません",
    eclipseSeries: "β と蝕の季節変動",
    eclipseApproximation:
      "各標本日の1周では太陽位置を固定。蝕は本影と半影の合計、半影の系列は本影を含まない時間です。極値は標本日の中から選びます。",
    createEclipseCase: "このケースを作る",
    eclipseLongest: "最長蝕の日",
    eclipseBetaMin: "β 最小の日",
    caseId: "新しいケースの ID",
    caseMode: "モード（任意）",
    noCaseMode: "指定しない",
    eclipseCaseDescription: "条件を再解析してケースを作ります。β と選んだ日付を保存します。",
    visibilityPerStation: "地上局ごとの可視",
    decaySeries: "高度の推移",
    decayApproximation:
      "1 周で平均した抗力（NRLMSISE-00）を積分した近円軌道の見積もりです。プリセットは太陽活動を全期間で一定にした条件、dated は宇宙天気の表から日付で引いた条件です。",
    decayWeatherTable: (first: string, last: string, fetched: string) =>
      `宇宙天気の表 ${first}〜${last}（取得 ${fetched}）を使いました。表の外は中程度のプリセットです。`,
    decayNoWeatherTable:
      "宇宙天気の表が無いので、プリセットの条件だけです（設定の外部データから取得できます）。",
  },
};
