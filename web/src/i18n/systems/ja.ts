import type { FillBasis } from "@/api/types";

/** 系の解析のページの文言（日本語） */
export const systemsJa = {
  systems: {
    eyebrow: "系の解析",
    unknown: (s: string) => `知らない系です: ${s}`,
    defaultDerivedCase: (name: string) => `条件選択が必要な解析の既定: ${name}`,
    defaultAttitude:
      "電力収支: 上段の時系列の姿勢則だけ既定ケースから選択。各モードの収支はそのモードの姿勢則で計算。",
    caseLabel: "解析ケース",
    defaultCase: "自動（必要な解析だけ既定）",
    recompute: "再計算",
    tabs: { results: "結果", breakdown: "内訳", settings: "設定", series: "時系列" },
    specs: {
      power: {
        description: "モード別の消費電力、発電、軌道上の収支とバッテリーの DoD",
      },
      attitude: {
        description: "RW のアンロード頻度、MTQ・RCS でのアンロード時間、センサの禁止角",
      },
      cdh: {
        description: "HK とミッションデータの発生量、保持に要るストレージと余裕",
      },
      comm: {
        description: "回線バジェット、地上局の可視時間、送信量と未送信データの蓄積",
      },
      thermal: { description: "多節点の定常・非定常解析と、動作温度範囲との比較" },
      structure: {
        description: "3D 配置、質量特性、準静的荷重と簡易な固有振動数",
      },
    },
    // KPI の名前は API（`KpiDef.label`・`label_en`）。ここは判定を数える KPI の数の文と、塗りの比の根拠（`FillBasis`）の文
    kpiTally: (checked: number, flagged: number) =>
      flagged === 0 ? `すべて OK（${checked}）` : `${flagged} / ${checked} が要確認`,
    kpiFill: {
      value: "値（割合）",
      used_over_limit: "上限に対する使用量",
      ok_share: "OK の判定の割合",
    } satisfies Record<FillBasis, string>,
    status: {
      ok: "実行済み",
      missing_inputs: "入力不足",
      failed: "失敗",
      running: "実行中…",
      error: "エラー",
    },
    steady: "定常",
    timeSeries: "時系列",
    missing: (n: number) => `足りない入力（${n}）`,
    openBreakdown: "内訳を開く",
    elapsed: "経過 [s]",
    partsAttrs: "部品 × この系の属性",
    partsAttrsCaption: "セルをクリックして直接編集（SetValue）。状態ごとの値は開いて状態ごとに直す",
    noParts: "この系に関係する部品がありません",
    part: "部品",
    kind: "種別",
    settingsTitle: (system: string) => `${system}の設定`,
    settingsCaption: (files: string) => `${files} の設定`,
    noSettings: "この系の設定はまだありません（プロジェクト / 設定 にすべての設定があります）",
    inputOf: (title: string) => `${title} に渡した入力`,
    noSeries: "時系列の結果はまだありません（時系列の解析が実行されると、ここにグラフを出します）",
    seriesSep: "・",
    seriesUnit: (u: string) => `［${u}］`,
    rangeSep: " 〜 ",
    heatTitle: "節点温度（定常）",
    heatLegendBefore: "枠: 黄 = 範囲の端から",
    heatLegendAfter: "以内、赤 = 範囲外",
    node: "節点",
    thermal3d: "温度で塗った 3D",
    thermal3dLegend: "各節点の最高温度（節点 ID = 部品 ID のもの）",
    layout3d: {
      title: "3D 配置",
      parts: (n: number) => `${n} 部品`,
      unplacedCount: (n: number) => ` · 未配置 ${n}`,
      com: " · 赤い点 = 重心",
      operation: "操作",
      translate: "移動",
      rotate: "回転",
      snap: "スナップ",
      empty: "機体座標が導出できた部品がありません（取付と位置を入れると出ます）",
      moved: (id: string) => `${id} を動かした`,
      placementOf: (id: string) => `${id} の配置`,
      hint: "箱を選ぶと、取付と位置・姿勢を数値で編集できます。",
      unplaced: "未配置",
      axes: "軸・法線・視線",
      axesLegend: "青 = スピン軸、水色 = MTQ 軸、黄 = 法線、赤 = 視線、紺 = 推力",
    },
  },
};
