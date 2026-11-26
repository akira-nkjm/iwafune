/** 共通の文言（ボタン・状態・読み込み中など） */
export const commonJa = {
  common: {
    resizeTree: "ツリーの幅を調整",
    expandAll: "すべて開く",
    collapseAll: "すべて閉じる",
    sortColumn: "昇順・降順・元の順に並べ替え",
    working: {
      elapsed: (seconds: number) => `${seconds} 秒`,
      computing: (name: string) => `${name}を計算中`,
      layingOutDiagram: "システム図の配置を計算中",
      drawing3d: "3D 表示を準備中",
    },
    loading: "読み込み中…",
    cancel: "キャンセル",
    apply: "適用",
    applying: "適用中…",
    add: "追加",
    remove: "削除",
    change: "変更",
    close: "閉じる",
    undo: "元に戻す",
    redo: "やり直す",
    undoTitle: "直前の編集を元に戻す（⌘Z / Ctrl+Z）",
    redoTitle: "元に戻した編集をやり直す（⇧⌘Z / Ctrl+Y）",
    save: "保存",
    saving: "保存中…",
    discard: "破棄",
    all: "すべて",
    retry: "再試行",
    /** 「主（補足）」の書き方（ID と表示名、名前と種類など。組み立ては lib/design.ts の withNote） */
    withNote: (main: string, note: string) => `${main}（${note}）`,
    ok: "OK",
    noData: "データなし",
    cannotLoadDesign: (msg: string) => `設計を読めません: ${msg}`,
    language: "言語",
    appearance: "外観",
    themeSystem: "OS に従う",
    themeLight: "ライト",
    themeDark: "ダーク",
    /** 図・3D の視点操作の案内（「?」の中身） */
    viewHelp: {
      label: "操作の案内",
      title: "視点の操作",
      zoom: "拡大・縮小",
      pan: "移動",
      rotate: "回転",
      fit: "全体",
      zoomHow:
        "トラックパッドはピンチ、マウスはホイール（Ctrl / ⌘ + ホイールも可）。カーソルの位置を中心に拡大します",
      panDiagramHow:
        "トラックパッドは 2 本指でスクロール、マウスは中ボタンのドラッグか、何もない所の左ドラッグ",
      pan3dHow:
        "トラックパッドは 2 本指でスクロール、マウスは中ボタン・右ボタンのドラッグか、Shift + 左ドラッグ",
      rotateHow:
        "1 本指のドラッグ、マウスは左ボタンのドラッグ。どの軸のまわりにも回せます（逆さまも可）。右下のキューブの面・辺・角を押すと、その向きから傾きなく見ます",
      fitDiagramHow: "左下の「全体を見る」ボタン、または 0 キー",
      fit3dHow: "右上のホームポジションのボタン（向きも初めに戻ります）",
    },
  },
};
