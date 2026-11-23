/** 足りない入力の行の操作（その場で値を入れる・部品を選ぶ・追加の画面へ行く）の文言（日本語） */
export const fixesJa = {
  fixes: {
    enterValue: "値を入れる",
    pickPart: "部品を選ぶ",
    addPart: "部品を追加",
    assignPart: "既存の部品を割り当てる",
    setOnPart: "部品に値を入れる",
    addItem: {
      mode: "モードを追加",
      maneuver: "マヌーバを追加",
      ground_station: "地上局を追加",
      comm_link: "通信の回線を追加",
      case: "解析ケースを追加",
    },
    value: {
      title: (path: string) => `${path} の値を入れる`,
      description: "打った値をそのまま保存し、足りない入力を解消します。",
      label: "値",
      dimension: (d: string) => `次元: ${d}`,
      scope: "書き込み先",
      part: "この部品の上書き",
      catalog: (item: string) => `カタログ品 ${item}`,
      scopeHint: "部品がカタログ品を選んでいます。品に書くと、同じ品を使う部品すべてに効きます。",
      summary: (path: string) => `${path} に値を入れる`,
      confirm: "保存",
    },
    pick: {
      title: (path: string) => `${path} の部品を選ぶ`,
      description: "設計にある部品から選んで入れます。",
      part: "部品",
      choose: "選んでください",
      none: "条件に合う部品がありません。部品を追加してください。",
      summary: (path: string, part: string) => `${path} を ${part} にする`,
      confirm: "入れる",
    },
  },
};
