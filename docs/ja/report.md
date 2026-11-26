# レポートの書き出し

`fune report export` は、設計データと解析から、設計レポートの表と図を書き出します。出力は**自分の Typst 文書**から取り込むファイルです。
本文は自分で書き、数値は iwafune が書きます。設計は何も変わらず、同じ設計からは毎回同じバイト列が出力されます（スナップショットテストで確かめています）。
PDF のビルド（`fune report build`）、LaTeX や CSV の断片、`reports/<ID>.toml`、GUI のボタンはありません。理由と、何があればこれらを足すかは[ロードマップ](roadmap.md)にあります。

## 使い方

```bash
fune report export                                  # every item, the default case, English, into exports/report/
fune report export --case hot_imaging --case cold_safe --lang ja
fune report export --item mass --item thermal-result --out build/report
typst compile exports/report/example.typ            # a complete example that includes every item
```

| オプション | 意味 |
|---|---|
| `--out <dir>` | 出力ディレクトリ。既定はプロジェクトルート下の `exports/report`。前回の書き出しの古い `figs/*.svg` は削除されます。 |
| `--case <ID>` | ケースに依存する項目に使う解析ケース（`cases/<ID>.toml`）。複数回指定できます。既定は解析の既定ケース（`analysis/common.toml` の `default_case`）を 1 回です。 |
| `--item <name>` | `mass`、`mode-power`、`link-budget`、`thermal-result`、`thermal-history`、`power-history` のいずれか。複数回指定できます。既定はすべてです。 |
| `--lang ja\|en` | 見出し、キャプション、注記の言語（既定は `en`）。設計に由来する名前（部品、モード、ケース）は、書いたとおりに出ます。 |

同じ操作が `POST /api/v1/report/export` で、本体は `{ items?, cases?, lang? }` です。サーバーはファイルの内容（`files[].path` と `files[].content`、`revision`、`unavailable`）を返し、何も書き込みません。
CLI は、同じサービス関数が返したものを書き込みます。そのため両者は同じバイト列を出力し、この書き出しは「設計ファイルは Op だけを通して書く」ルールの例外を必要としません（書くのは設計ファイルではなく派生ファイルです）。
`exports/` は派生物です。`.gitignore` に入れて（デモは入れています）そのつど作り直すか、レビューで正確な表が必要ならコミットします。

## ファイル

```
exports/report/
  data.json        every exported table, figure summary and citable value (SI values and display strings)
  iwafune.typ      the Typst module (the same file every time; no language-dependent text)
  example.typ      a document that includes everything that was exported
  figs/
    thermal-history-<case>.svg
    power-history-<case>.svg
```

`data.json` の抜粋（デモの質量表）を示します。

```json
{
  "schema": 1,
  "revision": "b3:5e54…",
  "project": "IWAFUNE 6U (demo)",
  "lang": "en",
  "items": ["mass", "mode-power", "link-budget", "thermal-result", "thermal-history", "power-history"],
  "cases": ["hot_imaging"],
  "tables": [
    {
      "item": "mass",
      "caption": "Mass budget",
      "columns": [
        { "header": "Item", "align": "left" },
        { "header": "Unit mass", "unit": "g", "align": "right" }
      ],
      "rows": [
        { "style": "group", "cells": [{ "text": "AOCS" }] },
        { "style": "body", "cells": [{ "text": "Reaction wheel, 10 mN·m·s class (RW_X)" }, …, { "text": "152", "si": 0.152 }] }
      ],
      "notes": ["Masses include the growth margin set per maturity. …", "Model revision: b3:5e54…"]
    }
  ],
  "figures": [{ "item": "thermal-history", "case": "hot_imaging", "file": "figs/thermal-history-hot_imaging.svg", "…": "…" }],
  "values": [{ "key": "part.RW_X.mass", "text": "152 g", "number": "152", "unit": "g", "si": 0.152 }]
}
```

行の `style` は `body`、`group`（見出し行）、`subtotal`、`total`、`margin` のいずれかです。セルの `si` は SI の数値で（テキストにはありません）、`detail` は値の下に出す短い注記です（電力表での部品の状態）。
入力が足りない表や図も、`unavailable` のテキスト（"Not available: missing inputs (…)"）を付け、行なしで出力されます。コマンドはそうした項目を "Not available because" の下に一覧し、書き出しの残りには影響しません。

## Typst から使う

```typst
#import "report/iwafune.typ": *

= Mass budget
The dry mass is #value("budget.mass.total"), of which the reaction wheels are #value("part.RW_X.mass") each.
#mass-table()

#link-budget(link: "uhf_down@TOKYO_GS")
#thermal-result(case: "hot_imaging")
#thermal-history(case: "hot_imaging")
#power-history(case: "hot_imaging")
```

| 関数 | 項目 |
|---|---|
| `#mass-table()` | サブシステムごとの項目と、個数、単体質量、質量、成長マージン、マージンを含む質量。小計、合計、上限、マージン。推進剤を数えるときは推進剤と湿質量の合計 |
| `#mode-power-table()` | モードごとの部品（成長マージンなしの平均電力。その下に状態と時間の割合）。成長マージンなしと込みのモードごとの合計。異なる場合は食中の合計。ピーク |
| `#link-budget(link: auto)` | リンク × 局ごとの入力表と結果表（`link: "<name>@<station>"` で 1 つを選びます）。キャプションにリンク、方向、局、周波数が出ます |
| `#thermal-result(case: auto)` | 定常ケース（高温、低温など）ごとのノードの、最小、ケースにわたる最大、許容範囲（動作時。なければ最初の種類）、ノードの判定 |
| `#thermal-history(case: auto)` | 図。すべてのノードの温度を 1 つの軸に、食の帯（同じケースの `power.orbit_generation` から）、許容範囲を破線で示します |
| `#power-history(case: auto)` | 図。発電と負荷、下限つきのバッテリー充電状態を、食の帯とともに示します |
| `#value("<key>", unit: true)` | 本文用に、単位つきの 1 つの値。キー: `part.<ID>.mass`（1 台あたり）、`budget.mass.{total,limit,margin}`、`kpi.<analysis>.<kpi>[@<case>]` |
| `#value-si("<key>")` | その値の SI の数値 |
| `revision` | ファイルを書き出した設計のリビジョン（`fune status` と比べて、古い表を見つけます） |

`case: auto` は、書き出したすべてのケースを返します。書き出していないケースを指定すると、その旨のメッセージでコンパイルが止まります。各関数は種類が `table` か `image` の `figure` を返すため、自分の文書の番号づけと `@ref` が使えます
（キャプションは表の上、図の下に付きます。章ごとの番号づけは、自分の文書の設定しだいです）。`data.json` と図のパスは `iwafune.typ` から解決されるため、モジュールはどの文書からも取り込めます
（文書がモジュールのディレクトリの下にないときは、`typst compile` に広い `--root` を渡します）。Typst 0.15.1 で確かめています。`fune report export` の実行に `typst` は要りません。

## 出力が従う規約

- 列見出しに単位を付け、1 列に 1 つの単位にします（`sat_units::format` の表示設定が、最小の値から選びます。電力と時間の列は最大の値から選びます）。数値は有効数字 3 桁です（`sat_units::format::format_number`）。
  リンクの表は、行ごとに量が違うので単位の列を持ちます。温度はシステム雑音温度（K）を除いて °C です。
- 表ごとに、成長マージンを含むかどうかと、マージンの測り方を示します（表の下の注記と、行のラベル）。
- ケースに依存する項目ではキャプションにケースを、すべての表と図の下の注記と `data.json` にモデルのリビジョンを書きます。
- 値は解析とバジェットの結果で、書き出しの中で再計算はしません。判定は解析の判定です。
- 図は素の SVG です。白い背景、細い軸、モノクロでも区別できる線のスタイル（色を先に使い、足りなくなると破線のパターン。`fune_report::palette`）、本物の `<text>` による文字、小数 2 桁に固定した座標を使います。
  幅は 640 単位です（レポートの幅で約 160 mm）。

## 実装の場所

| 責務 | 場所 |
|---|---|
| データモデル、`data.json` の形 | `fune_report::model` |
| 見出し、キャプション、注記（ja/en） | `fune_report::labels` |
| SVG のグラフ | `fune_report::{chart, legend, ticks, svg, palette}` |
| Typst モジュールと例 | `fune_report::typst`、`crates/app/report/src/template/iwafune.typ` |
| ファイルの配置 | `fune_report::files` |
| 解析から項目を作る処理 | `fune_service::report::{mass, power, link, thermal, history, values}` |
| CLI、HTTP | `fune report export`、`POST /report/export`（`fune_service::report::export`） |
