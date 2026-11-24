/** 足りない入力の行の操作（その場で値を入れる・部品を選ぶ・追加の画面へ行く）の文言（日本語） */
export const fixesJa = {
  fixes: {
    enterValue: "値を入れる",
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
      description: "値を打ってください。打った値をそのまま保存し、足りない入力を解消します。",
      descriptionChoice: "この設定が受け付ける値から選んでください。",
      choose: "選んでください",
      example: (e: string) => `例: ${e}`,
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
      action: {
        part: "部品を選ぶ",
        catalog_item: "カタログ品を選ぶ",
        comm_link: "回線を選ぶ",
        maneuver: "マヌーバを選ぶ",
      },
      title: (path: string) => `${path} に入れるものを選ぶ`,
      prompt: {
        part: () => "設計にある部品から選んでください。",
        catalog_item: (kinds: string) => `カタログから、種別 ${kinds} の品を選んでください。`,
        comm_link: () => "通信の設定にある下りの回線から選んでください。",
        maneuver: () => "日付つきの遷移（出発と到着の日）を持つマヌーバから選んでください。",
      },
      label: { part: "部品", catalog_item: "カタログ品", comm_link: "回線", maneuver: "マヌーバ" },
      choose: "選んでください",
      none: {
        part: () => "条件に合う部品がありません。先に部品を追加してください:",
        catalog_item: (kinds: string) =>
          `カタログに種別 ${kinds} の品がありません。先に追加してください:`,
        comm_link: () => "下りの回線がありません。先に下りの回線を追加してください:",
        maneuver: () => "日付つきの遷移を持つマヌーバがありません。先に追加してください:",
      },
      summary: (path: string, chosen: string) => `${path} を ${chosen} にする`,
      confirm: "入れる",
    },
  },
};
