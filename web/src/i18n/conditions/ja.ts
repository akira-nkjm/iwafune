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
    phase: "フェーズ",
    setPhase: "フェーズを選ぶ",
    epoch: "開始時刻",
    setEpoch: "開始時刻を入力する",
    caseEpoch: {
      title: (id: string) => `ケース ${id} の開始時刻`,
      description:
        "時系列の解析は、このケースの軌道をこの時刻から始めます（fune case set-epoch と同じ）。空にすると軌道の元期から始めます。",
      field: "開始時刻",
      hint: "ISO 8601（例: 2031-05-02T00:00:00Z）。1 回限りの期間を持つ軌道では、期間の中の時刻にします。",
      confirm: "保存",
      summary: (id: string, epoch: string) =>
        epoch ? `ケース ${id} の開始時刻を ${epoch} にする` : `ケース ${id} の開始時刻を外す`,
    },
    fromGeometry: {
      open: "最悪の日からケースを作る",
      title: "最悪の日からケースを作る",
      description:
        "フェーズの軌道のミッション幾何から、太陽距離の最大・最小と地球距離の最大の日を開始時刻にした 3 つのケースを作ります（既にあれば開始時刻などを更新。fune case from-geometry と同じ）。",
      phase: "フェーズ",
      prefix: "ケース ID の前置き（任意）",
      prefixHint:
        "<前置き>_sun_far・<前置き>_sun_near・<前置き>_earth_far。省略するとフェーズの ID（小文字）",
      mode: "モード（任意）",
      noMode: "指定しない（既存のケースのモードは変えない）",
      confirm: "ケースを作る",
      summary: (phase: string) =>
        phase ? `フェーズ ${phase} の最悪の日からケースを作る` : "最悪の日からケースを作る",
    },
    casePhase: {
      title: (id: string) => `ケース ${id} のフェーズ`,
      description:
        "選んだフェーズの軌道と姿勢則の上書きが、このケースの解析に使われます（fune case set-phase と同じ）。",
      field: "フェーズ",
      none: "フェーズなし（設計の軌道と姿勢則）",
      confirm: "保存",
      summary: (id: string, phase: string) =>
        phase ? `ケース ${id} のフェーズを ${phase} にする` : `ケース ${id} のフェーズを外す`,
    },
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
    stations: {
      add: "地上局を足す",
      addTitle: "地上局を足す",
      addDescription:
        "conditions/ground/<ID>.toml を作ります。帯域ごとの性能（アンテナ利得・雑音温度など）は、できたあとに設定で書きます。",
      addSummary: (id: string) => `地上局 ${id} を足す`,
      addConfirm: "足す",
      label: "表示名（任意）",
      latitude: "緯度",
      longitude: "経度",
      altitude: "標高（任意）",
      minElevation: "最低仰角（任意）",
      quantityHint: "緯度・経度は必須です。単位を付けて書きます（例: 35.7 deg）",
      noValues: "この地上局にはまだ値がありません",
      rename: (id: string) => `地上局 ${id} の名前を変える`,
      renameTitle: (id: string) => `地上局 ${id} の名前を変える`,
      renameDescription: "通信の解析が使う地上局の一覧（analysis.comm.stations）も書き換えます。",
      newId: "新しい ID",
      renameConfirm: "変える",
      renameSummary: (id: string, to: string) => `地上局 ${id} → ${to}`,
      remove: (id: string) => `地上局 ${id} を消す`,
      removeTitle: (id: string) => `地上局 ${id} を消す`,
      removeDescription: (file: string) => `${file} を消します。取り消しは変更履歴から行えます。`,
      inUse:
        "通信の解析（analysis.comm.stations）がこの地上局を挙げているので消せません。先に設定から外してください。",
      removeSummary: (id: string) => `地上局 ${id} を消す`,
      removeConfirm: "消す",
    },
    decaySeries: "高度の推移",
    decayApproximation:
      "1 周で平均した抗力（NRLMSISE-00）を積分した近円軌道の見積もりです。プリセットは太陽活動を全期間で一定にした条件、dated は宇宙天気の表から日付で引いた条件です。",
    decayWeatherTable: (first: string, last: string, fetched: string) =>
      `宇宙天気の表 ${first}〜${last}（取得 ${fetched}）を使いました。表の外は中程度のプリセットです。`,
    decayNoWeatherTable:
      "宇宙天気の表が無いので、プリセットの条件だけです（設定の外部データから取得できます）。",
  },
};
