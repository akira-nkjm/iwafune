# CLI ガイド

`fune` コマンドは設計プロジェクト（git で管理する TOML ファイルのディレクトリ）を読み書きし、解析とバジェットを実行して、結果を表示します。このページは作業ごとに構成しています。各作業について、コマンド、デモ衛星での実際の例、コマンドが何を変えるかを示します。すべてのコマンドの一覧はページの最後にあります。

例では[クイックスタート](quickstart.md)と同じく、デモプロジェクト（`fixtures/demo-6u`）のコピーを使います。ほとんどの編集は `--dry-run` で示します。`--dry-run` は差分を表示するだけで何も書き込まないので、コピーを変えずに試せます。長い出力は `…` で省略しています。パスはプロジェクトからの相対パスで示します。

`fune --help` はコマンドの一覧を、`fune <command> --help` は 1 つのコマンドのオプションを表示します。このページとヘルプの内容が食い違うときは、ヘルプが正しい内容です。

目次

- [グローバルオプション](#グローバルオプション)
- [プロジェクトを作って調べる](#プロジェクトを作って調べる)
- [値を読み書きする](#値を読み書きする)
- [部品とカタログ](#部品とカタログ)
- [モードとシーケンス](#モードとシーケンス)
- [ケース、フェーズ、形態](#ケースフェーズ形態)
- [要求](#要求)
- [通信リンクと地上局](#通信リンクと地上局)
- [マヌーバ](#マヌーバ)
- [ネット、ハーネス、定義、図の配置](#ネットハーネス定義図の配置)
- [解析とバジェットを実行する](#解析とバジェットを実行する)
- [Op JSON を適用する](#op-json-を適用する)
- [検査、整形、取り消し](#検査整形取り消し)
- [設計文書を書き出す](#設計文書を書き出す)
- [データセット](#データセット)
- [共有カタログ](#共有カタログ)
- [エージェントスキル、更新、GUI](#エージェントスキル更新gui)
- [コマンド一覧](#コマンド一覧)

## グローバルオプション

次のオプションはすべてのコマンドで使えます。

| オプション | 意味 |
|---|---|
| `-C, --dir <DIR>` | プロジェクト。`fune.toml` があるディレクトリか、その下のディレクトリです。既定は現在のディレクトリです。`fune` は `fune.toml` を上のディレクトリへ向かって探します |
| `--json` | 表の代わりに機械可読の JSON を表示します |
| `--color <auto\|always\|never>` | 色の指定です。`auto` は、出力が端末でないとき、または `NO_COLOR` が設定されているときは色を付けません |
| `-h, --help` | コマンドのヘルプ |
| `-V, --version` | `fune` のバージョン |

設計に書き込むコマンドにはさらに 2 つのオプションがあります。

| オプション | 意味 |
|---|---|
| `--dry-run` | 変更されるファイルの差分を表示し、何も書き込みません |
| `--base-revision <REV>` | このリビジョンのあとで設計が変わっていたら、書き込みません（終了コード 3）。ほかの編集を上書きしてはならないスクリプト向けです |

`fune fmt` と `fune undo`・`fune redo` はファイルを変えますが編集ではないので、この 2 つのオプションを取りません。読むだけのコマンドにも、書き込むものがないので不要です。

### 終了コード

| コード | 意味 |
|---|---|
| 0 | 成功 |
| 1 | 設計または結果が不合格: チェックのエラー、不足している入力、実行の失敗、違反の判定 |
| 2 | 入力が不正、または見つからない |
| 3 | 競合: 設計が途中で変わった、または ID がすでにある |

警告ではコマンドは失敗しません。`fune check --deny-warnings` だけは例外です。

### JSON 出力

`--json` はすべてのコマンドで使えます。例として、プロジェクトの件数を示します。

```console
$ fune status --json
{
  "name": "iwafune-6u-demo",
  "root": "my-demo/.",
  "revision": "b3:5e54b14f210acafe01e4f5c5e43246567e0e32e81bd8f04f1978c00f7365bafd",
  "branch": "main",
  "schema": 2,
  "counts": {
    "parts": 38,
    "groups": 11,
    "modes": 4,
    "nets": 8,
    "harnesses": 2,
    "cases": 3
  },
  …
}
```

`--dry-run --json` で書き込むと、対応するコマンドが `cli` 配列に入って返ります。GUI の操作に対応するコマンドを知りたいときに便利です。

## プロジェクトを作って調べる

プロジェクトを作るには、`fune init [DIR]` を使います。ディレクトリと、その中に最小のプロジェクトを作ります。`fune.toml`、バジェットのファイル、モード 1 つ、`.gitignore` ができます。`git init` は実行しません。プロジェクト名はパスの最後の部分です。`--name` で指定もできます。すでに `fune.toml` があるディレクトリは拒否します。

```console
$ fune init my-sat
✔ Created project my-sat: my-sat/…
  fune.toml
  budget/allocation.toml
  budget/policy.toml
  modes/SAFE.toml
  .gitignore
Next:
  cd my-sat
  fune status
```

概要とチェックには`fune status` と `fune check` を使います。`fune status` はプロジェクトが持つものを数え、診断の概要を示します。`fune check` は設計を検証します。

```console
$ fune status
iwafune-6u-demo
my-demo/.  branch main  rev b3:5e54b14f2
parts 38  groups 11  modes 4  nets 8  harnesses 2  cases 3
✔ Check: no problems
```

一覧と詳細の表示です。ほとんどの名詞に `ls` があります。

```console
$ fune part ls
 ID           Subsystem    Kind                  Definition          Count   Mass     Power         Operating temp
───────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 BAT          power        battery               local/bat_2s2p          1    395 g             —   0 °C … 45.0 °C
 BUS_FRAME    structure    structure             —                       1    620 g             —   not set
 CAM          payload      camera                local/cam_ms_5m         1    920 g   6.80 W (on)   0 °C … 35.0 °C
 …
```

`fune part show <ID>` は1 つの部品のすべてをサブシステムごとにまとめて表示します。値とその出典、配置、ポートです。`fune catalog show <ITEM>`、`fune phase show <ID>`、`fune configuration show <ID>` も、それぞれの対象について同じことをします。ほかの一覧は `fune mode ls`、`fune net ls`、`fune harness ls`、`fune case ls`、`fune source ls`、`fune requirement ls`、`fune maneuver ls`、`fune kind ls`、`fune attr ls`、`fune units ls`、`fune setting ls` です。

## 値を読み書きする

`fune get` は値を読み、出どころを示します。`fune set` は値を書きます。`fune unset` は上書きを消します。

```console
$ fune get RW_X.mass
152 g  Override, em_test_2026_10, Measured
$ fune get RW_X.power --state on
600 mW  Catalog, Datasheet
```

値のパスは `<部品>.<属性>` です。状態ごとの値は `<部品>.<属性>.<状態>` です。値が状態ごとのときに `--state` を付けずに `fune get` を実行すると、すべての状態を表示します。`off 0 W / on 650 mW  Catalog, Datasheet` のようになります。

`fune set` は値を変え、出典と注記を指定すれば、それも書きます。

```console
$ fune set RW_X.mass "160 g" --source em_test_2026_10 --note "Flight unit" --dry-run
--- a/parts/aocs/RW/RW_X.toml
+++ b/parts/aocs/RW/RW_X.toml
@@ -5,10 +5,10 @@
 point = "RW_SLOT_X"

 [attr.mass]
-value       = 0.152
-unit        = "kg"
+value       = 160.0
+unit        = "g"
 from.source = "em_test_2026_10"
-from.note   = "EM measurement"
+from.note   = "Flight unit"

 [ports.PWR]
 net = "P12V"
✔ dry-run: 1 files would change (nothing written)
```

書いた単位（`g`）はそのままファイルに残ります。`fune` が書くファイルは常に正規の形なので、編集の直後でも `fune fmt --check` は成功します。

| オプション | 意味 |
|---|---|
| `--source <ID>` | 値の出典（`sources/` の ID）。存在する ID でなければなりません |
| `--at <TEXT>` | 文書内の場所。たとえば `"p.12 table 3"` |
| `--note <TEXT>` | 自由な注記 |
| `--min <VALUE>`, `--max <VALUE>` | 不確かさの下限と上限。値と同じ単位で書きます（異なる単位は拒否されます） |

既存の値に `fune set` を実行すると、値のレコード全体が置き換わります。値に出典と注記があり、それらをもう一度指定しないと、消えてしまいます。

```console
$ fune set RW_X.mass "160 g" --dry-run
--- a/parts/aocs/RW/RW_X.toml
+++ b/parts/aocs/RW/RW_X.toml
@@ -4,11 +4,8 @@
 on    = "PANEL_MZ"
 point = "RW_SLOT_X"

-[attr.mass]
-value       = 0.152
-unit        = "kg"
-from.source = "em_test_2026_10"
-from.note   = "EM measurement"
+[attr]
+mass = { value = 160.0, unit = "g" }

 [ports.PWR]
 net = "P12V"
✔ dry-run: 1 files would change (nothing written)
```

出典の記録を残すには、`--source` をもう一度指定します。`fune unset` は上書きを消し、カタログの値に戻します。

```console
$ fune unset RW_X.mass --dry-run
--- a/parts/aocs/RW/RW_X.toml
+++ b/parts/aocs/RW/RW_X.toml
@@ -4,12 +4,6 @@
 on    = "PANEL_MZ"
 point = "RW_SLOT_X"

-[attr.mass]
-value       = 0.152
-unit        = "kg"
-from.source = "em_test_2026_10"
-from.note   = "EM measurement"
-
 [ports.PWR]
 net = "P12V"

✔ dry-run: 1 files would change (nothing written)
```

### 値の書式

値はテキストで入力します。`fune` は属性や設定の定義に照らしてそれを読みます。型を指定する必要はありません。GUI の入力欄も同じ読み方をするので、ここで通る値は GUI でも通ります。

| 値の型 | 書き方 | 例 |
|---|---|---|
| 量 | 数値と単位。単位は属性の次元と一致しなければなりません | `"620 g"`, `"550 km"`, `"7.4 V"`, `"-20 degC"` |
| 単位のない数（割合、効率） | 数値だけ | `0.28` |
| 範囲 | `min .. max` と、最後に 1 つの単位。両端に単位を付けてもよいですが、同じ単位でなければなりません | `"-10 .. 50 degC"` |
| 3 成分のベクトル | 空白かカンマで区切った 3 つの数値。最後に単位を付けられます | `"0 0 1"`, `"100 100 20 mm"` |
| クォータニオン | 4 つの数値: w x y z | `"1 0 0 0"` |
| 曲線 | `x:y` の組と、括弧の中の単位（x の単位、`->`、y の単位） | `"0:480 10:360 20:150 (km^2/s^2 -> kg)"` |
| 真偽値 | `true` か `false` | `true` |
| 列挙、テキスト | 語またはテキストをそのまま | `sun_pointing`, `"Wheel X"` |
| 一覧の設定 | カンマで区切った語、または文字列の TOML 配列 | `"BAT_A, BAT_B"`, `'["BAT_A", "BAT_B"]'` |
| 時刻 | `HH:MM` | `"10:30"` |

実数の設定に入力した数値は実数として保存されます。`fune set conditions.orbit.eccentricity 0` は `eccentricity = 0.0` と書きます。`cases.<ID>.orbits` のような整数の設定には、整数を入力します。

`--dry-run` を使うと、値がどう保存されるかが分かります。範囲やベクトルの形を知るには、これがいちばん早い方法です。

```console
$ fune set RW_X.temp_op "-10 .. 50 degC" --dry-run
…
+[attr]
+temp_op = { min = -10.0, max = 50.0, unit = "degC" }
$ fune set RW_X.spin_axis "0 1 0" --dry-run
…
+[attr]
+spin_axis = { x = 0.0, y = 1.0, z = 0.0 }
```

単位を使う前に確かめるには、`fune units parse` で次元と SI の値を、`fune units ls` ですべての単位を表示します。

```console
$ fune units parse "18 mN*m*s"
 Input         Dimension         SI
────────────────────────────────────────────────
 18.0 mN·m·s   AngularMomentum   0.0180 m²·kg/s
```

テキストが誤っていると、何も書き込まれず、終了コードは 1 か 2 になります。

| 入力 | 結果 |
|---|---|
| 次元が違う量: `fune set RW_X.mass "160 m"` | "Unit m is Length, but attribute mass is Mass"（`unit.dimension`） |
| 単位のない量: `fune set RW_X.mass 160` | "Unit  is Dimensionless, but attribute mass is Mass" |
| 数値でないテキスト: `fune set RW_X.mass abc` | "Cannot parse unit: abc (Cannot read value abc)" |
| 2 つの数値のベクトル: `fune set RW_X.spin_axis "0 1"` | "Cannot parse unit: 0 1 …" |
| `true` でも `false` でもない真偽値: `fune set conditions.orbit.sun_synchronous maybe` | "Invalid input: Cannot read value maybe" |
| 値と異なる単位の `--min`: `--min "1 s"` | "The unit of --min / --max must be the same as the value: g" |
| 未知の出典: `--source nope` | "Source nope is not in sources/"（`ref.source`） |
| 未知の属性や部品: `RW_X.nothing`、`NOPE.mass` | "attribute definition not found: nothing"、"target not found: Part NOPE" |
| 列挙にない語: `analysis.aocs.law.NOMINAL.law foo` | "unknown variant foo, expected one of sun_pointing, nadir_pointing, inertial" |
| テンソル | テキストでは入力できません。`fune apply` で JSON として渡します |
| 要求が持つ限度: `fune setting set analysis.power.dod_max 0.3` | 判定基準なので要求が持つこと、`fune requirement set` で編集する要求を示すメッセージが出ます |

ファイルとしては有効なので受け付けられ、`fune check` が警告する誤りもあります。たとえば `--min "100 g" --max "90 g"` は `value.uncertainty_order`（「The uncertainty does not satisfy min ≤ value ≤ max」）になります。自信がない編集のあとは、`fune check` を実行してください。

### 設定とレコード

`fune set` は設定も書きます。パスは `conditions.`、`analysis.`、`budget.` で始まります。たとえば `fune set conditions.orbit.altitude "550 km"` です（`fune setting set conditions.orbit.altitude "550 km"` と同じです）。`fune setting ls` はすべての設定を値とファイルつきで表示します。`fune setting unset <PATH>` は設定を 1 つ消します。

解析ケースのフィールドは `cases.<ID>.<field>` で書きます。フィールドは `label`、`description`、`mode`、`phase`、`side`（`nominal`、`min`、`max`）、`thermal_case`、`beta`、`orbits`、`sun_distance`、`epoch` です。変わるのは 1 つのフィールドだけなので、ケースの編集にはこの方法を使います（[ケース](#ケースフェーズ形態)）。

```console
$ fune set cases.hot_imaging.beta "45 deg" --dry-run
--- a/cases/hot_imaging.toml
+++ b/cases/hot_imaging.toml
@@ -2,5 +2,5 @@
 mode         = "MISSION"
 side         = "max"
 thermal_case = "hot"
-beta         = { value = 60.0, unit = "deg" }
+beta         = { value = 45.0, unit = "deg" }
 phase        = "NOMINAL_OPS"
```

`fune unset cases.<ID>.<field>` は、フィールドを 1 つ消します。

## 部品とカタログ

| コマンド | 内容 |
|---|---|
| `fune part add <ID> --catalog <ITEM> [--group <G>] [--count N] [--label L] [--mode-state S]` | カタログの項目から部品を追加します。`--kind <KIND>` を使うと、種類だけの空の部品を作ります。すべてのモードに行が追加され、既定は `off` です |
| `fune part rm <ID>` | 部品とモードの中のその行を削除します。ほかのファイルがまだ参照しているときは、エラーの一覧つきで拒否されます |
| `fune part rename <OLD> <NEW>` | 部品の名前を変えます。参照はすべて書き換わります |
| `fune part mv <ID> <TARGET>` | 部品を別のサブシステム、グループ、組立品へ移します。`aocs/RW` は、`aocs` の下のグループ `RW` へ移し、グループがなければ作ります |
| `fune part rename-group` | フォルダの名前を変えます |
| `fune part place <ID> --on <PART> --point <P> --position "x y z mm" --rotation "rx ry rz deg"` | 搭載先と、相対位置・姿勢を設定します。`--body-pos` と `--body-rot` は機体の座標を使います。`--configuration <ID>` は形態の姿勢を書きます |
| `fune part point <PART> <POINT> --position "…"` | 部品の搭載点を追加します（`--rm` で削除） |
| `fune catalog add <ID> --kind <KIND> [--label …] [--maturity …] [--from-part <PART>]` | カタログの項目を追加します。ID には由来のカタログ名が付きます: `local/rw_30mNms` |
| `fune catalog set <ITEM> <PATH> <VALUE>` | 属性かフィールドを書きます: `mass`、`power.on`、`label`、`manufacturer`、`maturity` |
| `fune catalog unset`, `rm`, `rename`, `mv` | 値の削除、項目の削除（部品が使っている間は拒否されます）、名前の変更、カタログの別のディレクトリへの移動 |

部品を追加して、変わるファイルを見てみます。

```console
$ fune part add FSS_2 --catalog local/fss_60 --group aocs --dry-run
--- a/modes/DOWNLINK.toml
+++ b/modes/DOWNLINK.toml
@@ -5,6 +5,7 @@
 [states]
 CAM     = { state = "off" }
 EPT     = { state = "off" }
+FSS_2   = { state = "off" }
 GPS     = { state = "on" }
 …
--- a/parts/aocs/FSS_2.toml
+++ b/parts/aocs/FSS_2.toml
@@ -0,0 +1 @@
+catalog = "local/fss_60"
✔ dry-run: 5 files would change (nothing written)
```

新しい部品はカタログの項目を指す 1 行のファイルで、質量、消費電力、制限値は項目から取ります。部品はどのモードにも `off` の行を持つので、モードのファイルも 4 つ変わります。名前の変更と移動は、参照をすべて整合させます。名前の変更をドライランすると、変わるファイルがすべて分かります。

```console
$ fune part rename GPS GPS_1 --dry-run
--- a/analysis/thermal.toml
+++ b/analysis/thermal.toml
@@ -32,7 +32,7 @@
 BAT      = ["BAT", "HTR_BAT"]
 BUS      = ["BUS_FRAME"]
 CAM      = ["CAM", "HTR_CAM"]
-ELEC     = ["OBC", "PCDU", "GPS", "UHF_TRX", "STX"]
+ELEC     = ["OBC", "PCDU", "GPS_1", "UHF_TRX", "STX"]
…
✔ dry-run: 6 files would change (nothing written)
```

ほかのものがまだ参照している部品の削除は、拒否されます。拒否の内容は削除によって生じる診断の一覧です（たとえば「Mount target part `CAM` is not defined」）。何も書き込まれません。参照しているファイルを先に削除するか付け替えるか、`fune part rename` を使ってください。

## モードとシーケンス

```console
$ fune mode table
 Part      NOMINAL           MISSION               DOWNLINK          SAFE
─────────────────────────────────────────────────────────────────────────────────────
 CAM       off               standby (on 50.0 %)   off               off
 EPT       off               off                   off               off
 GPS       on                on                    on                off
 HTR_BAT   off (on 20.0 %)   off (on 10.0 %)       off (on 10.0 %)   off (on 40.0 %)
 …
```

括弧の中の文字列はデューティで、別の状態にある時間の割合です。

| コマンド | 内容 |
|---|---|
| `fune mode add <ID> [--label …] [--from <MODE>]` | モードを追加します。`--from` は、別のモードの表をコピーします |
| `fune mode set <MODE> <PART> <STATE> [--duty state=fraction] [--active N] [--spare-state S]` | モードの中の部品の状態を設定します。`--duty tx=0.1` は、時間の 10 % を `tx` で過ごすという意味です |
| `fune mode unset <MODE> <PART>` | モードから部品の行を削除します |
| `fune mode duration <MODE> --max-duration "90 min"` | 最大継続時間を書きます（フラグを省くと削除します） |
| `fune mode timing <MODE> --when sunlit` | 最長継続時間の評価でそのモードを使う時機を書きます。別の形は `--start eclipse_exit --offset "5 min"` です |
| `fune mode move`, `rename`, `rm` | 並べ替え、名前の変更、削除。ケース、シーケンス、フェーズがまだ使っているモードの削除は拒否され、メッセージにそれらが挙がります |
| `fune mode view <SYSTEM>` | `power`、`cdh`、`thermal`、`aocs` のモードごとの値を、日照と食に分けて表示します |

例: ダウンリンク中はカメラをスタンバイにします（ドライラン）。

```console
$ fune mode set DOWNLINK CAM standby --dry-run
--- a/modes/DOWNLINK.toml
+++ b/modes/DOWNLINK.toml
@@ -3,7 +3,7 @@
 max_duration = { value = 15.0, unit = "min" }

 [states]
-CAM     = { state = "off" }
+CAM     = { state = "standby" }
```

`fune mode view power` は各モードでの各部品の消費電力を表示します。GUI の Power ページが表示する表です。

```console
$ fune mode view power
Sunlit
 Part                            NOMINAL    MISSION    DOWNLINK   SAFE
───────────────────────────────────────────────────────────────────────────
 CAM                                  0 W     4.20 W        0 W        0 W
 EPT                                  0 W        0 W        0 W        0 W
 GPS                               900 mW     900 mW     900 mW        0 W
 …
```

シーケンスは継続時間つきのモードを順に並べたものです。電力の解析は使用中のシーケンスをシミュレーションします。`fune sequence ls` は一覧を表示し、`fune sequence set` は 1 つを丸ごと書き、`fune sequence rm` は 1 つを削除します。

```console
$ fune sequence ls
 Name       In use   Anchor                    Segments
───────────────────────────────────────────────────────────────────────────────────────────────────────────────
 default    yes                                NOMINAL 76.0 min, MISSION 5.00 min, NOMINAL 5.00 min, DOWNLINK 9.60 min
 downlink            eclipse_exit + 10.0 min   NOMINAL 60.0 min, DOWNLINK 15.0 min, NOMINAL 20.6 min
 imaging             eclipse_exit              NOMINAL 40.0 min, MISSION 10.0 min, NOMINAL 35.6 min, DOWNLINK 10.0 min
$ fune sequence set imaging_day --segments "NOMINAL=76 min,MISSION=5 min" --anchor eclipse_exit --dry-run
--- a/analysis/power.toml
+++ b/analysis/power.toml
@@ -57,3 +57,14 @@
…
+[sequences.imaging_day]
+anchor = { from = "eclipse_exit" }
…
```

シーケンスを使用中にするには、`fune setting set analysis.power.active_sequence imaging_day` を実行します。

## ケース、フェーズ、形態

軌道と環境の設定は `fune set conditions.<…>` で書きます。[最初の衛星のチュートリアル](first-satellite.md#11-解析を実行して不足している入力を埋める)を参照してください。

ケースは`fune case set <ID>` で作ります。すでにあるケースにはレコード全体を置き換えます。指定しなかったフィールドは削除され、コマンドがそれを一覧で表示します。1 つのフィールドだけを変えるには、`fune set cases.<ID>.<field>` を使います。

```console
$ fune case ls
 ID              Name                          Modes     Phase         File
────────────────────────────────────────────────────────────────────────────────────────
 cold_safe       Cold (safe mode)              SAFE      NOMINAL_OPS   cases/cold_safe.toml
 eclipse_worst   Worst-case eclipse (β = 0°)   NOMINAL   NOMINAL_OPS   cases/eclipse_worst.toml
 hot_imaging     Hot (imaging)                 MISSION   NOMINAL_OPS   cases/hot_imaging.toml
$ fune case set my_case --mode NOMINAL --label "My case" --dry-run
--- a/cases/my_case.toml
+++ b/cases/my_case.toml
@@ -0,0 +1,2 @@
+label = "My case"
+mode  = "NOMINAL"
✔ dry-run: 1 files would change (nothing written)
```

ケースのフィールドは`--label`、`--description`、`--mode`、`--phase`、`--side nominal|min|max`、`--thermal-case`、`--beta "60 deg"`、`--orbits`、`--sun-distance "0.7 AU"`、`--epoch <ISO 8601>` です。ほかのケースのコマンドは`rename`、`rm`、`set-phase`、`unset-phase`、`set-epoch`、`unset-epoch` と、軌道の最悪の日からケースを作る `from-eclipse`、`from-sun-distance`、`from-geometry` です。

まだ参照されているケースの削除は拒否され、メッセージが何が参照しているかを示します。

```console
$ fune case rm hot_imaging
✖ Invalid input: Analysis case `hot_imaging` cannot be removed: referred to by `default_case` in analysis/common.toml; `quantify.cases` of requirement(s) R_STORAGE. Change those first
```

フェーズ、モード、形態、出典、リンク、地上局、カタログの項目も同じです。拒否のメッセージはそれを参照しているファイルやエンティティを挙げるので、先に何を変えればよいかが分かります。

フェーズの一覧です。

```console
$ fune phase ls
 #   ID            Name                          Duration   Modes                           Orbit      Law   Cases
─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 1   LEOP          Launch and early operations     30.0 d   SAFE NOMINAL
 2   NOMINAL_OPS   Nominal operations               730 d   NOMINAL MISSION DOWNLINK SAFE                    cold_safe eclipse_worst hot_imaging
 3   DISPOSAL      Disposal                         180 d   SAFE NOMINAL                    override
```

`fune phase add|set|rm|rename` は見出しのフィールドを書きます。フェーズの軌道と姿勢則の上書きは`set-field`、`unset-field`、`copy-orbit`、`unset-orbit`、`unset-law` で書きます。`fune phase show <ID>` は 1 つのフェーズを表示します。`fune phase rm NOMINAL_OPS` は、ケースがそのフェーズを参照している間は拒否されます。

形態（configuration）は収納状態や展開状態のような、部品の姿勢に名前を付けた重ね合わせです。`fune configuration ls` は形態の一覧と、それぞれが動かす部品を表示します。`add`、`set`、`rm`、`rename`、`unset-pose` で編集します。姿勢は `fune part place <PART> --configuration <ID>` で書きます。

```console
$ fune configuration ls
 ID       Name     Overridden parts
────────────────────────────────────
 STOWED   Stowed   SAP_PY
$ fune configuration show STOWED
Stowed (STOWED)
  description  Launch configuration: the deployable paddle SAP_PY is folded flat onto the +X face …
  overridden   SAP_PY
  file         configurations/STOWED.toml
```

## 要求

`fune requirement add <ID>` には、記述、量（次元: `Mass`、`Power`、`Temperature` など）、向き（`--kind at_most|at_least|equal|range`）、境界値が必要です。確認するものは`--budget mass|power|data` か、`--analysis <ID>` と `--check <key>` の組です。

```console
$ fune requirement add R_TEST --statement "Test" --quantity Mass --kind at_most --value "7 kg" --budget mass --dry-run
--- a/requirements/R_TEST.toml
+++ b/requirements/R_TEST.toml
@@ -0,0 +1,9 @@
+statement = "Test"
+quantity  = "Mass"
+
+[constraint]
+kind  = "at_most"
+value = { value = 7.0, unit = "kg" }
+
+[verify]
+budget = "mass"
✔ dry-run: 1 files would change (nothing written)
requirement.source_missing

  ⚠ A requirement should name its source (a source ID in sources/)
```

末尾の警告は `--source <ID>` を求めています。そのほかのオプションは`--statement-en`、`--label`、`--tolerance`、`--min`/`--max`（範囲）、`--all-cases true|false`、`--cases a,b`、`--input mission_duration` です。`fune requirement set` は要求全体を置き換えます。`fune requirement rm` は要求を削除します。

適合表は判定を集めたものです。違反があると、終了コード 1 で終わります。

```console
$ fune requirement compliance
 Verdict          ID               Requirement                                                   Constraint   Cases        Detail
 ✔                R_BAT_TEMP       The battery shall stay above 0 degC in every analysis case.      ≥ 0 °C   cold_safe, eclipse_worst, hot_imaging   cold_safe · BAT minimum temperature …
 ? not verified   R_DEPLOY         The antenna shall be deployed within 30 minutes after separation.   ≤ 30.0 min   default   no analysis check or budget linked
 …
 ✖                R_STORAGE        The data held on board shall not exceed the OBC storage of 8 GB.   ≤ 8.00 GB   eclipse_worst, hot_imaging   hot_imaging: Total 54.1 GB against limit 8.00 GB
16 requirements: 11 ok, 1 warning, 1 violation, 3 not verified
```

列はここでは省略しています。読むべきなのは最後の行です。

## 通信リンクと地上局

地上局は `conditions/ground/` にあるファイルです。`fune ground add` は位置を書きます（`--latitude`、`--longitude`、`--altitude`、`--min-elevation`）。電波の帯域ごとの性能は `fune set conditions.ground.<ID>.link.<band>.<field>` で書きます。-90 から 90 deg の外の緯度や、0 から 90 deg の外の最小仰角は拒否されます。

```console
$ fune ground add TEST_GS --latitude "35.7 deg" --longitude "139.7 deg" --dry-run
--- a/conditions/ground/TEST_GS.toml
+++ b/conditions/ground/TEST_GS.toml
@@ -0,0 +1,2 @@
+latitude  = { value = 35.7, unit = "deg" }
+longitude = { value = 139.7, unit = "deg" }
$ fune ground add BAD --latitude "95 deg" --longitude "139.7 deg" --dry-run
✖ Nothing was written because of validation errors (1)
setting.range

  × Latitude must be from -90 to 90 deg
  help: parameter: conditions.ground.BAD.latitude  conditions/ground/BAD.toml:1
```

リンクは `analysis/comm.toml` のセクションで、送受信機の部品と地上局の帯域を結びます。`fune link add <ID>` はリンクが読み込まれるために必要なフィールドを書きます。損失と変調方式は `fune set analysis.comm.link.<name>.<field>` で設定します。

```console
$ fune link add x_down --direction down --part STX --band s_band --dry-run
--- a/analysis/comm.toml
+++ b/analysis/comm.toml
@@ -35,3 +35,8 @@
…
+[link.x_down]
+direction = "down"
+part      = "STX"
+band      = "s_band"
$ fune set analysis.comm.link.s_down.pointing_loss "1.5 dB" --dry-run
…
-pointing_loss       = { value = 1.0, unit = "dB" }
+pointing_loss       = { value = 1.5, unit = "dB" }
```

`fune ground rename|rm` と `fune link rename|rm` は、参照（解析が使う地上局の一覧、ダウンリンクの選択、要求のチェック）を整合させます。要求のチェックが参照しているリンクや地上局の削除は、拒否されます。

```console
$ fune link rm s_down
✖ Invalid input: Link `s_down` cannot be deleted: `verify.check` of requirement(s) R_LINK_S, R_PFD refers to it. Update or remove those requirements first
```

解析が使う地上局は、一覧 `analysis.comm.stations` です。

## マヌーバ

マヌーバは `maneuvers/` にあるファイルで、実行順に並びます。マヌーバには区分（`insertion`、`station_keeping`、`unloading`、`deorbit`、`other`）と方式（`impulsive`、`finite_burn`、`low_thrust`、`propagated`）が必要です。

```console
$ fune maneuver ls
 #   ID        Name                                Category    Method       Thruster   Target
───────────────────────────────────────────────────────────────────────────────────────────────────────
 1   RAISE     Orbit raise 550 → 600 km            Insertion   Low thrust   EPT        altitude 600 km
 2   DEORBIT   Deorbit (lower perigee to 250 km)   Deorbit     Impulsive    EPT        perigee_altitude 250 km
$ fune maneuver add TCM --category station_keeping --method impulsive --delta-v "15 m/s" --dry-run
--- a/maneuvers/TCM.toml
+++ b/maneuvers/TCM.toml
@@ -0,0 +1,3 @@
+category = "station_keeping"
+method   = "impulsive"
+delta_v  = { value = 15.0, unit = "m/s" }
```

`--thruster`、`--after`、`--altitude`、`--perigee-altitude`、`--apogee-altitude`、`--inclination`、`--body` は目標を記述します。`fune maneuver set` はマヌーバを置き換え、`rm` は削除し（あとのマヌーバは、その前のマヌーバにつなぎ直されます）、`rename` は名前を変え、`set-window` は日付探索の結果から、天体間の遷移の日付を書きます。

## ネット、ハーネス、定義、図の配置

これらのコマンドは使う機会が少なめです。編集はどれも `--dry-run` を受け付けます。

ネットは電気的な接続です。ネットには種類（`power`、`data`、`rf`）と、レール電圧などの任意のフィールドがあります。

```console
$ fune net add P28V --kind power --rail "28 V" --limit "2 A" --dry-run
--- a/nets/P28V.toml
+++ b/nets/P28V.toml
@@ -0,0 +1,3 @@
+kind  = "power"
+rail  = { value = 28.0, unit = "V" }
+limit = { value = 2.0, unit = "A" }
✔ dry-run: 1 files would change (nothing written)
```

`fune net connect <PART>.<PORT> <NET>` はポートをネットに接続し、`disconnect` は外します。`fune net set` と `unset` はネットのフィールドを変え、`replace` はネットを書き直します。`rename` と `rm` は参照を整合させます。

ハーネスのセグメントは2 つのポートの間の物理的なケーブルです。`fune harness set` はセグメント全体を書きます（残したいフィールドはすべて指定します）。

```console
$ fune harness set H_TEST --from OBC.CAN1 --to GPS.DATA --nets CAN_BUS --length "0.2 m" --dry-run
--- a/harness/H_TEST.toml
+++ b/harness/H_TEST.toml
@@ -0,0 +1,4 @@
+from   = "OBC.CAN1"
+to     = "GPS.DATA"
+nets   = ["CAN_BUS"]
+length = { value = 0.2, unit = "m" }
✔ dry-run: 1 files would change (nothing written)
```

属性と種類の定義です。`fune attr add` は新しい属性を定義します（型、次元、使える単位、列挙なら記号）。`fune kind add` は部品の種類を定義します（状態、必須と任意の属性、ポートのひな型）。どちらにも `ls`、`replace`、`rename`、`rm` があります。組み込みの定義は置き換えられません。

```console
$ fune attr add wheel_count --type scalar --dimension Dimensionless --label "Wheel count" --dry-run
--- a/defs/attribute/wheel_count.toml
+++ b/defs/attribute/wheel_count.toml
@@ -0,0 +1,3 @@
+label     = "Wheel count"
+type      = "scalar"
+dimension = "Dimensionless"
$ fune kind add cold_plate --extends component --states "off,on" --required "mass" --dry-run
--- a/defs/kind/cold_plate.toml
+++ b/defs/kind/cold_plate.toml
@@ -0,0 +1,4 @@
+label    = "cold_plate"
+extends  = "component"
+states   = ["off", "on"]
+required = ["mass"]
```

熱テーブルです。`fune thermal tables` は節点間の熱結合（使っている値とその出どころ）、外面のコーティング、節点の温度を表示します。GUI の「Couplings and surfaces」タブが表示するものと同じです。読むだけです。編集は `fune set analysis.thermal.conductive.<A>.<B>`（および `radiative`、`coating`、`surface`）で行います。

```console
$ fune thermal tables
Couplings
 Kind         Node A     Node B     Value        Source
─────────────────────────────────────────────────────────────
 conductive   BAT        BUS        0.0500 W/K   Written value
 conductive   BAT        PANEL_MZ   0.0300 W/K   Written value
 conductive   BUS        ELEC         2.50 W/K   Mount default
 …
```

確認済みの指摘です。幾何の検査は部品の箱どうしの重なりと、プルームや視野が部品にかかることを警告します。指摘が箱モデルによる既知の見かけ上のものなら、理由をつけて確認済みにします。指摘は一覧に残りますが、警告はしなくなります。`fune acknowledged rm` は確認済みを取り下げます。

```console
$ fune acknowledged add interference SAP_PY SP_PY --reason "Hinge edge of the box model" --dry-run
--- a/analysis/structure.toml
+++ b/analysis/structure.toml
@@ -22,4 +22,4 @@
 finding = "interference"
 first   = "SAP_PY"
 second  = "SP_PY"
-reason  = "Hinge edge: the paddle box and the solar panel box meet at the hinge line, where the real parts only touch"
+reason  = "Hinge edge of the box model"
```

（デモではこの組はすでに確認済みなので、ドライランでは理由が置き換わります。）

図の配置です。GUI のシステム図は節点を自動で配置します。`fune view set <DIAGRAM> <NODE> --pos X,Y` は節点を固定し、`fune view edge-set` は線の経路を固定し、`fune view ls <DIAGRAM>` は固定したものを一覧にし、`fune view reset <DIAGRAM>` は自動配置に戻します。配置は `view/` に保存されます。GUI で節点をドラッグしたときも、同じものが書かれます。

```console
$ fune view set electrical RW_X --pos 420,120 --dry-run
--- a/view/electrical.toml
+++ b/view/electrical.toml
@@ -0,0 +1,2 @@
+[nodes]
+RW_X = { x = 420, y = 120 }
✔ dry-run: 1 files would change (nothing written)
```

プロジェクトの設定です。`fune project set name <NAME>` と `fune project set label <TEXT>` は、`fune.toml` の `[project]` テーブルを書きます。`fune project unset label` はラベルを削除します。

## 解析とバジェットを実行する

`fune analyze ls` はすべての解析を領域、ID、名前、種類つきで一覧にします。`fune analyze <ID>` は 1 つを実行します。解析は[解析のページ](../analyses/index.md)で説明しています。

```console
$ fune analyze ls
 Area         ID                            Name                                           Kind          Input
───────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 aocs         aocs.rw_unloading             RW unloading frequency                         time series   built from design
 aocs         aocs.unload_time              Momentum unloading time                        steady        built from design
 …
$ fune analyze comm.link_budget
✔ comm.link_budget ran
     KPI                 Value
─────────────────────────────────
 ✔   Link margin (min)   3.05 dB
 Verdict   Target   Item                                 Detail
─────────────────────────────────────────────────────────────────────────────────────────────────────
 ✔ ok               s_down@TOKYO_GS link margin          Margin 5.95 dB (required at least 3.00 dB)
 …
Use --json for the full result
```

| オプション | 意味 |
|---|---|
| `--case <ID>` | 解析ケースを指定して実行します |
| `--configuration <ID>` | 形態（姿勢）から入力を組み立てます |
| `--ops <FILE>` | 保存していない編集（Op の JSON ファイル、標準入力なら `-`）をメモリ上で適用してから解析します。何も書き込みません |
| `--input <FILE>` | 入力全体を与える JSON ファイル。設計から組み立てません |
| `--detail` | 重い任意の出力も返します（`--json` で） |
| `--json` | 完全な結果: KPI、チェック、入力、出力、時系列、不足している入力 |

入力が不足しているとき、実行が失敗したとき、チェックが違反のときは、終了コードが 1 になります。入力が不足していると、コマンドは不足している入力を理由つきで 1 つずつ挙げ、それを直す `fune set` コマンドを示します（[チュートリアルのステップ 11](first-satellite.md#11-解析を実行して不足している入力を埋める)）。

バジェットは`fune budget mass|power|data` で表示します。`--case` を付けると、電力とデータをそのケースのモードだけで数えます。`--ops` は保存していない編集を適用します。

```console
$ fune budget power
Power budget
power.mode_load  mode DOWNLINK
     Row             Current   With growth   Allocated   Margin
─────────────────────────────────────────────────────────────────
 ✔   AOCS             4.32 W        4.58 W      6.00 W    1.42 W
 …
 ✖   Communication    10.3 W        11.3 W      3.00 W   -8.27 W
 …
✔ Total 18.7 W  limit 24.0 W (system margin 15.0 %)  margin 1.68 W
```

書き込まずに仮定を試すには、変更を JSON の Op で記述して、結果を見ます。

```console
$ cat ops.json
[{"op":"set_value","part":"CAM","attr":"mass","value":{"type":"scalar","data":{"value":1.2,"unit":"kg"}}}]
$ fune budget mass --ops ops.json | tail -2
✔ Dry total 6.57 kg  limit 12.0 kg (system margin 10.0 %)  margin 4.23 kg
✔ Total with propellant 6.64 kg  margin 4.16 kg  (propellant from propulsion.propellant)
```

ファイルは変わりません。GUI は保存していない下書きに同じ仕組みを使っています。

## Op JSON を適用する

`fune apply <FILE>` はOp のファイルを適用します（標準入力は `-`）。GUI が送るのと同じ JSON で、多くの編集をまとめて行うスクリプトに向いています。

```console
$ fune apply ops.json --dry-run
--- a/parts/payload/CAM.toml
+++ b/parts/payload/CAM.toml
@@ -8,6 +8,9 @@
 [pose]
 position = { x = 0.0, y = -60.0, z = -90.0, unit = "mm" }

+[attr]
+mass = { value = 1.2, unit = "kg" }
+
 [ports.PWR]
 net = "P5V"

✔ dry-run: 1 files would change (nothing written)
```

## 検査、整形、取り消し

```bash
fune check                   # exit 1 on an error
fune check --deny-warnings   # warnings fail too
fune fmt --check             # list files not in canonical form; exit 1 if any
fune fmt                     # rewrite them
```

`fune fmt` はファイルを正規の形にします。キーの順序が固定され、`=` がそろい、TOML 1.0 になります。`fune` が書くファイルはすでに正規の形なので、`fmt --check` が報告するのは、手で編集したファイルだけです。`fune fmt` は Op ではないので、`fune undo` では取り消せません。

### 取り消しと履歴

```console
$ fune history
   1          fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"  1 file(s)
(keeps the last 100 edits; `fune undo` / `fune redo` move along this list)
$ fune undo
  modified parts/payload/CAM.toml
✔ Undid: fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"  rev b3:5e54b14f210a
```

`fune history` は編集を古い順に一覧にします。`fune undo` は直前の編集が変えたファイルを元に戻します。`fune redo` は取り消した編集をもう一度適用します。新しい編集をすると、やり直しの側は消えます。そのあとでエディタ、git、`fune fmt`、カタログの取得でファイルが変わっていると、取り消しは拒否されます（終了コード 3）。履歴は GUI と共有されます。GUI では変更履歴のポップオーバーに Undo と Redo のボタンがあります。

## 設計文書を書き出す

`fune report export` は自分の Typst 文書に使う設計報告書の表と図を書き出します。設計は何も変えません。

```console
$ fune report export --out exports/report
✔ Wrote 5 files to exports/report (revision b3:5e54b14f210a…)
```

`data.json`、`iwafune.typ`、`example.typ` と、履歴の項目があれば SVG の図を入れた `figs/` ディレクトリを書きます。オプションは次のとおりです。

| オプション | 意味 |
|---|---|
| `--out <DIR>` | 出力先のディレクトリ。既定はプロジェクトのルート下の `exports/report` です |
| `--case <ID>` | ケースに依存する項目を書き出す解析ケース。複数なら繰り返します。省くと、解析の既定のケースを 1 回使います |
| `--item <NAME>` | 書き出す項目: `mass`、`mode-power`、`link-budget`、`thermal-result`、`thermal-history`、`power-history`。複数なら繰り返します。省くと 6 項目すべてです |
| `--lang <ja\|en>` | 見出し、図表のキャプション、注記の言語。既定は `en` です |

```console
$ fune report export --out exports/mass --item mass --lang ja
✔ Wrote 3 files to exports/mass (revision b3:5e54b14f210a…)
```

表の項目を 1 つだけ書き出すと、ファイルは 3 つで、図はありません。未知の項目は有効な項目の一覧つきで拒否されます。詳細は[レポートの書き出し](../../report.md)にあります。

## データセット

```console
$ fune data status
 ID                 State               Size      Loaded   Location
─────────────────────────────────────────────────────────────────────────────────────────
 de440              ✔ SHA-256 matches   20.7 MB   yes      ~/.cache/iwafune/data/de440.bin
 …
 space_weather      not fetched               —   —        ~/.cache/iwafune/data/SW-All.csv
Fetch: fune data fetch space_weather
```

`fune data status` は任意のデータセット（天体暦、宇宙天気、放射線データ）を、状態、サイズ、場所つきで一覧にします。`fune data fetch <ID>` は 1 つをダウンロードします。`fune data space-weather <DATE>` は、日付の F10.7 と Ap を調べ、値がダウンロードした表から来たのか、プリセットから来たのかを示します。表とコマンドは[インストール](install.md#任意のデータセット)を参照してください。

## 共有カタログ

共有カタログは`catalog/` ディレクトリ（と、必要なら `sources/`）を持つ git リポジトリです。使うには次のようにします。

```console
$ fune catalog source add shared --git file:///srv/shared-catalog --rev main
  modified fune.toml
✔ Wrote 1 files  rev b3:d13744bad82d
catalog.not_locked

  × The commit of shared catalog `shared` is not in catalog.lock. Run `fune catalog fetch` to fetch it, then commit catalog.lock
…
Added shared catalog `shared` to fune.toml (not fetched yet). Run `fune catalog fetch`, then commit catalog.lock
$ fune catalog fetch
 Catalog   git                          rev    commit         Previous commit
───────────────────────────────────────────────────────────────────────────────
 shared    file:///srv/shared-catalog   main   668d80223e1a   —
+ shared/rw_10
✔ Rewrote catalog.lock (commit it)
$ fune catalog ls
 ID              Subsystem   Kind             Name                                          Model   Attributes
───────────────────────────────────────────────────────────────────────────────────────────────────────────────
 local/bat_2s    power       battery          Li-ion 2S pack                                                 3
 …
 shared/rw_10    aocs        reaction_wheel   Reaction wheel, 10 mN*m*s class (fictional)                    1
```

リポジトリのパスは省略しています。実際には `https` か `ssh` の URL を使ってください。URL は`git` が取得できるものなら何でも使えます。認証は git の設定に従います。

項目は `<名前>/<ID>` という名前で、読み取り専用です。ほかの項目と同じように使えます。`fune part add RW_X --catalog shared/rw_10 --group aocs` のようにします。`fune.toml` と `catalog.lock` をコミットしてください。コミットは `catalog.lock` に固定され、取得したファイルは `.fune/` の下にキャッシュされます。

リポジトリの新しいコミットへ移るにはカタログを更新します。コマンドは何が変わったかを示します。

```console
$ fune catalog update shared
 Catalog   git                          rev    commit         Previous commit
───────────────────────────────────────────────────────────────────────────────
 shared    file:///srv/shared-catalog   main   90cdb29c85f2   668d80223e1a
~ shared/rw_10
    attr.mass: 150 g → 160 g
✔ Rewrote catalog.lock (commit it)
```

`fune catalog source rm shared` はカタログを削除します。部品がその項目を使っている間は拒否されます。

```console
$ fune catalog source rm shared
✖ Invalid input: Items of catalog `shared` are used by part RW_T so it cannot be removed. Re-point or delete the users first
```

`fune catalog source add-local <NAME> --path <DIR>` は、このプロジェクトの中にある別のカタログを追加します。ディレクトリは `vendor_parts` のような名前で、`parts` のようにプロジェクトの構成が使う名前は使えません。項目は `<NAME>/<ID>` という名前になります。ディレクトリは最初の項目を追加したときに作られます。

```console
$ fune catalog source add-local vendor --path vendor_parts --dry-run
--- a/fune.toml
+++ b/fune.toml
@@ -9,5 +9,9 @@
 name = "local"
 path = "catalog"

+[[catalog]]
+name = "vendor"
+path = "vendor_parts"
+
 [modes]
 order = ["NOMINAL", "MISSION", "DOWNLINK", "SAFE"]
✔ dry-run: 1 files would change (nothing written)
```

CI では検査の前に、ロックを変えずに、ロックされたコミットを取得します。

```console
$ fune catalog fetch --locked
 Catalog   git                          rev    commit         Previous commit
───────────────────────────────────────────────────────────────────────────────
 shared    file:///srv/shared-catalog   main   668d80223e1a   (same)
✔ catalog.lock is unchanged
```

`--locked` は`catalog.lock` がないとき、または `fune.toml` と合わないときに失敗します。非公開のリポジトリでは`fune catalog fetch --locked` の前に、git がトークンで認証するようにします。

```yaml
      - uses: actions/checkout@v4
      - run: git config --global url."https://x-access-token:${{ secrets.CATALOG_TOKEN }}@github.com/".insteadOf "https://github.com/"
      - run: fune catalog fetch --locked
      - run: fune check
```

## エージェントスキル、更新、GUI

エージェントスキルです。`fune skill` はAI のコーディングエージェントに設計の扱い方を教えます。`fune skill show` はバイナリに同梱している指示文（`SKILL.md`）を表示します。`fune skill install` はそれをプロジェクトに書き込みます。

```console
$ fune skill show | head -4
---
name: fune
description: Edit and analyze satellite conceptual-design data with the `fune` CLI (a git-managed directory of TOML with a fune.toml at its root). Use it whenever you add or change parts, modes, ...
metadata:
$ fune skill install
✔ Installed the fune skill …: my-demo/.claude/skills/fune/SKILL.md
$ fune skill install
✔ The fune skill is up to date (…): my-demo/.claude/skills/fune/SKILL.md
```

もう一度インストールしても、ファイルが最新なら何もせず、別のバージョンが書いたものなら置き換えます。自分で編集していると、`--force` を付けない限り拒否されます（終了コード 3）。`--skills-dir <PATH>` を使うと、別のスキルのディレクトリにインストールします。指示文は編集は CLI だけで行うこと、`fune check` と `fune fmt --check` を実行すること、単位を書くことを求めています。

プログラムを更新します。`fune self-update --check` は、現在のバージョンと最新のバージョンを表示し、何も変えません。`fune self-update` は最新のリリースをインストールし、`fune self-update --version <MAJOR.MINOR.PATCH>` はそのバージョンをちょうどインストールします。`curl` と `tar` が必要で、プロジェクトの外でも動きます。[インストール](install.md#更新)を参照してください。

```console
$ fune self-update --check
Checking the latest release...
✔ fune … is already the latest version
```

GUI を起動します。`fune gui` はプロジェクトでローカルの Web サーバーを起動し、ブラウザーを開きます。[GUI ガイド](gui.md)を参照してください。

| オプション | 意味 |
|---|---|
| `--port <PORT>` | ポート。既定は 3000 で、使用中で指定もないときは、次に空いているポートを使います |
| `--host <HOST>` | 待ち受けるアドレス。既定は `127.0.0.1` で、すべてのインターフェースなら `0.0.0.0` です |
| `--allow-host <NAME>` | `Host` ヘッダーで受け付ける名前か IP（繰り返せます）。別の名前でサーバーに接続するときに必要です。ほかの `Host` のリクエストには 403 を返します |
| `--no-open` | ブラウザーを開きません |

## コマンド一覧

コマンドごとに 1 行で、`fune --help` と各サブコマンドのヘルプから取っています。どのコマンドも[グローバルオプション](#グローバルオプション)を受け付けます。編集には `--dry-run` と `--base-revision` が使えます。

| コマンド | 目的 |
|---|---|
| `fune init [DIR]` | 新しいプロジェクトを作ります |
| `fune check` | 設計データを検査します（エラーがあれば終了コード 1）。`--deny-warnings` |
| `fune gui` | GUI（ローカルの HTTP サーバー）を起動し、ブラウザーで開きます |
| `fune status` | プロジェクトの概要 |
| `fune part ls\|show\|add\|rm\|mv\|rename\|rename-group\|place\|point` | 部品 |
| `fune set <PATH> <VALUE>` | 値を設定します（`--source`、`--at`、`--note`、`--min`、`--max`） |
| `fune get <PATH>` | 値を表示します（`--state`） |
| `fune unset <PATH>` | 上書きした値を消して、カタログの値に戻します。解析ケースのフィールドを 1 つ消すこともできます |
| `fune attr ls\|add\|replace\|rename\|rm` | 属性の定義 |
| `fune kind ls\|add\|replace\|rename\|rm` | 種類の定義 |
| `fune mode ls\|table\|view\|add\|rm\|rename\|move\|duration\|timing\|set\|unset` | 運用モード |
| `fune thermal tables` | 熱結合、外面のコーティング、コーティング（読み取り専用。編集は `fune set\|unset analysis.thermal.…`） |
| `fune sequence ls\|set\|rm` | 電力解析の、名前つきのモードシーケンス |
| `fune acknowledged add\|rm` | 確認済みの幾何の指摘: 箱モデルによる既知の見かけ上の重なりや、プルーム・視野の干渉 |
| `fune net ls\|add\|set\|unset\|replace\|rm\|rename\|connect\|disconnect` | ネット（電気的な接続） |
| `fune setting ls\|set\|unset` | `conditions/`、`analysis/`、`budget/` の設定 |
| `fune catalog ls\|show\|fetch\|update\|add\|set\|unset\|rm\|rename\|mv\|source` | カタログの項目と、カタログの由来（`source add\|add-local\|rm`） |
| `fune case ls\|set\|from-eclipse\|from-sun-distance\|from-geometry\|set-phase\|unset-phase\|set-epoch\|unset-epoch\|rm\|rename` | 解析ケース |
| `fune source ls\|set\|rm\|rename\|mv` | 出典の登録簿 |
| `fune harness ls\|set\|rm\|rename` | ハーネスのセグメント |
| `fune maneuver ls\|add\|set\|rm\|rename\|set-window` | マヌーバ |
| `fune phase ls\|show\|add\|set\|rm\|rename\|set-field\|unset-field\|copy-orbit\|unset-orbit\|unset-law` | ミッションのフェーズ |
| `fune configuration ls\|show\|add\|set\|rm\|rename\|unset-pose` | 形態: 収納状態や展開状態など、部品の姿勢に名前を付けた重ね合わせ |
| `fune ground add\|rm\|rename` | 地上局 |
| `fune link add\|rm\|rename` | 通信リンク |
| `fune project set\|unset` | プロジェクトの設定（`fune.toml` の `name`、`label`） |
| `fune requirement ls\|compliance\|add\|set\|rm` | 要求と適合表 |
| `fune view ls\|set\|rm\|reset\|edge-set\|edge-reset` | 図の配置（システム図の節点の位置） |
| `fune units ls\|parse\|format` | 単位 |
| `fune fmt` | 設計ファイルを整形します。`--check` は報告だけです |
| `fune history` | `fune undo` と `fune redo` が使う編集履歴を、古い順に一覧にします |
| `fune undo` | 直前の編集を取り消します |
| `fune redo` | 取り消した編集をやり直します |
| `fune analyze ls\|<ID>` | 解析を実行します（`--case`、`--configuration`、`--input`、`--ops`、`--detail`） |
| `fune budget mass\|power\|data` | バジェット（`--case`、`--ops`） |
| `fune report export` | Typst 文書向けの報告書の表と図（`--out`、`--case`、`--item`、`--lang`） |
| `fune data status\|fetch\|space-weather` | 外部データ（天体暦、宇宙天気、放射線データ） |
| `fune self-update` | このバイナリを最新のリリースで置き換えます（`--check`、`--version`） |
| `fune skill show\|install` | `fune` の使い方を AI エージェントに教える指示文を表示、インストールします |
| `fune apply <FILE>` | Op の JSON を適用します（GUI と同じ形式） |
