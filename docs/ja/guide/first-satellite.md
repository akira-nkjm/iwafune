# 最初の衛星を作る

このチュートリアルでは空のディレクトリから質量バジェット、電力バジェット、解析の判定を 1 つ出すところまで進みます。プロジェクトを作り、バス構造にオンボードコンピューター、バッテリー、太陽電池パネルを載せた最小の衛星を組み立てます。途中で、不足している入力を 1 つずつ埋める方法も確かめます。

作業はすべてコマンドラインで行います。コマンドを使うと、何が変わるのかがそのまま見えるからです。どのコマンドにも GUI の対応する操作があります。場所は [GUI ガイド](gui.md)に書いています。どのステップでも `fune gui` を実行して、結果を GUI で見られます。所要時間は 30 分ほどです。

`fune` コマンドと `git` が必要です（[インストール](install.md)）。値は架空のものなので、自分の値に置き換えてください。長い出力は `…` で省略しています。リビジョンのハッシュ（`rev b3:…`）はステップによっては手元の表示と異なります。

## 1. プロジェクトを作る

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
$ cd my-sat
$ git init
```

`fune init` は `git init` を実行しません。自分で実行してください。新しいプロジェクトには 5 つのファイルがあります。

- `fune.toml` にはプロジェクト名とカタログの一覧が入ります。プロジェクトには`catalog/` ディレクトリにある `local` という専用のカタログが 1 つあります。
- `budget/allocation.toml` と `budget/policy.toml` には、配分と余裕の方針が入ります。
- `modes/SAFE.toml` は最初のモードです。
- `.gitignore` はキャッシュのディレクトリ `.fune/` を git の対象から外します。

```console
$ fune status
my-sat
my-sat/.  branch main  rev b3:2ef6b7d3b
parts 0  groups 0  modes 1  nets 0  harnesses 0  cases 0
⚠ Check: 1 warnings
$ fune check
budget.limit_unset

  ⚠ No requirement sets the mass limit (add an at_most requirement with verify.budget = "mass")
   ╭─[budget/allocation.toml:1:1]
 1 │ [mass]
   · ───┬──
   ·    ╰── requirement[budget/mass]
 2 │ system_margin = 0.1
   ╰────
  help: parameter: requirement[budget/mass]  budget/allocation.toml:1
⚠ Check: 1 warnings
```

`fune check` の結果にはすでに警告が 1 つ出ているはずです。質量の限度を決めるものがない、という警告です。iwafune では限度は要求として書くので、ステップ 3 で追加します。その前に、引用する出典が必要です。

## 2. 出典を登録する

すべての値に、出どころを書いておくようにします。出典は `sources/` にある小さなファイルです。種類は `placeholder`、`judgement`、`analysis`、`datasheet`、`measured` の 5 つです。2 つ登録します。

```console
$ fune source set sys_judgement --kind judgement --label "Team estimate" --date 2026-11-01
  created sources/sys_judgement.toml
✔ Wrote 1 files  rev b3:50d890c01f76
$ fune source set ds_vendor --kind datasheet --label "Vendor datasheets (fictional)" --ref "docs/datasheets/"
  created sources/ds_vendor.toml
✔ Wrote 1 files  rev b3:a9b9490b4858
```

各コマンドは `created` とファイル名、リビジョンつきの `✔ Wrote 1 files` を表示するはずです。警告が残る編集のあとは、`fune` が同じ警告をもう一度表示します。ここでは繰り返しを省いています。どのステップの結果も `fune fmt --check` と `fune check` で確かめられます。このチュートリアルのどのコマンドのあとでも、前者は何も表示せず、後者は問題なしと表示するか、次のステップで解消する警告だけを表示します。

## 3. 質量の要求を追加する

```console
$ fune requirement add R_MASS --statement "The satellite total mass shall not exceed 3 kg." \
    --quantity Mass --kind at_most --value "3 kg" --budget mass --source sys_judgement
  created requirements/R_MASS.toml
✔ Wrote 1 files  rev b3:3e6153e4dbef
$ fune check
✔ Check: no problems
```

`fune check` が問題なしと表示するはずです。要求には何を制約するか（`--quantity Mass`）、向き（`at_most`）、値、何で確認するか（`--budget mass`、つまり質量バジェット）を書きます。警告は消えました。この形の要求が、質量バジェットの限度になります。

## 4. カタログを埋める

カタログの項目はバッテリーの型式のような部品の種類を一度だけ記述するものです。部品は項目を指します。3 つの項目を作ります。`local/` はカタログの名前で、あとは項目名です。

```console
$ fune catalog add local/bat_2s --kind battery --label "Li-ion 2S pack" --maturity off_the_shelf
$ fune catalog set local/bat_2s mass "250 g" --source ds_vendor --at "p.3 table 1"
$ fune catalog set local/bat_2s capacity "40 Wh" --source ds_vendor
$ fune catalog set local/bat_2s voltage "7.4 V" --source ds_vendor
```

各コマンドはファイル名つきの `created` か `modified` と、`✔ Wrote 1 files` を表示します。以降は省略します。値は数値、単位（`g`、`Wh`、`V`）、出典（`--source`）、必要なら文書内の場所（`--at`）でできています。単位は検査され、次元が合わない単位は拒否されます。たとえば `fune catalog set local/bat_2s capacity "40 kg"` は、「Unit kg is Mass, but attribute capacity is Energy」というエラーで止まり、何も書き込みません。

種類ごとに必要な属性は種類の一覧で確かめます。

```console
$ fune kind ls
 ID                    role                  States                   Required attributes
─────────────────────────────────────────────────────────────────────────────────────────────────────
 battery               battery                                        capacity, voltage
 obc                   obc                   off / on                 power
 solar_panel           solar_panel                                    area, cell_efficiency, normal
 structure             structure
 ...
```

必須の属性が足りないと、`fune check` が属性ごとに警告します。（実際の表にはここに示した以上の行と、`Definition` という列がもう 1 つあります。）次に、太陽電池パネルとコンピューターを作ります。

```console
$ fune catalog add local/sp_body --kind solar_panel --label "Body-mounted solar panel" --maturity modified
$ fune catalog set local/sp_body mass "180 g" --source ds_vendor
$ fune catalog set local/sp_body area "0.06 m^2" --source ds_vendor
$ fune catalog set local/sp_body cell_efficiency 0.28 --source ds_vendor
$ fune catalog set local/sp_body normal "0 0 1" --source ds_vendor
$ fune catalog add local/obc_a --kind obc --label "On-board computer" --maturity off_the_shelf
$ fune catalog set local/obc_a mass "90 g" --source ds_vendor
$ fune catalog set local/obc_a power.off "0 W" --source ds_vendor
$ fune catalog set local/obc_a power.on "0.65 W" --source ds_vendor
$ fune catalog ls
 ID              Subsystem   Kind          Name                       Model   Attributes
─────────────────────────────────────────────────────────────────────────────────────────
 local/bat_2s    power       battery       Li-ion 2S pack                              3
 local/obc_a     cdh         obc           On-board computer                           2
 local/sp_body   power       solar_panel   Body-mounted solar panel                    4
```

`fune catalog ls` に、属性が 3 個、2 個、4 個の項目が 3 つ並ぶはずです。次の 3 点を押さえてください。

- `cell_efficiency` のように単位のない量は、数値だけで書きます。`normal` のようなベクトルは3 つの数値 `"0 0 1"` で書きます。
- `power.on` は状態ごとの値です。コンピューターの消費電力はオフかオンかで変わります。パスは `<属性>.<状態>` の形です。
- `--maturity` は設計がどれだけ新しいかを表します。バジェットは成熟度ごとの成長余裕を加えます。`budget/policy.toml` では`off_the_shelf` が 5 %、`modified` が 10 %、`new` が 20 % です。

## 5. モードを定義する

モードは公称やセーフのような、衛星の運用の仕方です。新しい部品は既存のすべてのモードに行を持つので、部品を追加する前に公称モードを追加します。`created modes/NOMINAL.toml` と表示されるはずです。

```console
$ fune mode add NOMINAL --label "Nominal"
  created modes/NOMINAL.toml
✔ Wrote 1 files  rev b3:4c6979c3a743
```

## 6. 部品を配置する

部品はカタログの項目（または種類だけ）を衛星の中で 1 回使ったものです。最初は構造です。構造にはカタログの項目がないので、`--kind` を使います。

```console
$ fune part add BUS --kind structure --group structure --label "Bus structure"
  created parts/structure/BUS.toml
✔ Wrote 1 files  rev b3:fb7bb40a8ec3
attr.missing

  ⚠ Required attribute `mass` is not set (kind `component`)
  help: parameter: BUS.mass  parts/structure/BUS.toml
$ fune set BUS.mass "600 g" --source sys_judgement --note "Placeholder until CAD"
  modified parts/structure/BUS.toml
✔ Wrote 1 files  rev b3:b29db88132ab
$ fune part point BUS FACE_PZ --position "0 0 50 mm"
  modified parts/structure/BUS.toml
✔ Wrote 1 files  rev b3:6d08861fb0cc
```

`--group` は部品を `parts/` の下のディレクトリに入れます。最上位のディレクトリがサブシステムです（`power`、`cdh`、`structure`）。`fune part point` はバスに搭載点を追加します。搭載点はほかの部品が取り付く、名前のついた場所です。次に、カタログから 3 つの部品を作って配置します。

```console
$ fune part add BAT --catalog local/bat_2s --group power
$ fune part add SP_PZ --catalog local/sp_body --group power --label "Solar panel +Z"
$ fune part add OBC --catalog local/obc_a --group cdh
  modified modes/NOMINAL.toml
  modified modes/SAFE.toml
  created parts/cdh/OBC.toml
✔ Wrote 3 files  rev b3:c310d1dba050
$ fune part place SP_PZ --on BUS --point FACE_PZ
$ fune part place BAT --on BUS --position "0 0 -20 mm"
$ fune part place OBC --on BUS --position "0 0 10 mm"
$ fune part ls
 ID      Subsystem   Kind          Definition      Count   Mass     Power         Operating temp
─────────────────────────────────────────────────────────────────────────────────────────────────
 BAT     power       battery       local/bat_2s        1    250 g             —   not set
 BUS     structure   structure     —                   1    600 g             —   not set
 OBC     cdh         obc           local/obc_a         1   90.0 g   650 mW (on)   not set
 SP_PZ   power       solar_panel   local/sp_body       1    180 g             —   not set
4 parts
```

`fune part ls` に 4 つの部品が並び、`fune check` に警告が出ないはずです。`OBC` を追加すると、モードのファイルが 2 つ変わりました。部品はすべてのモードに `off` の行を持つからです（`--mode-state` で別の状態を選べます）。部品の質量はカタログの項目から来ます。位置は搭載先を基準にした、部品自身の座標系の値です。GUI の Layout ページを開くと、箱が見えます。

<!-- screenshot: /design/layout — バスに 3 つの部品を配置した 3D ビューと、開いた Parts ツリー -->
*バスと 3 つの部品を表示した Layout ページ。*

## 7. 1 つの部品で値を上書きする

カタログの値は既定値です。部品はそれを上書きでき、上書きには不確かさを付けられます。コンピューターの質量が 95 g で、90 g から 110 g の間だとします。

```console
$ fune set OBC.mass "95 g" --source sys_judgement --note "Margin for the connector" --min "90 g" --max "110 g"
  modified parts/cdh/OBC.toml
✔ Wrote 1 files  rev b3:b1f70c6a7c72
$ fune get OBC.mass
95.0 g (90.0 g … 110 g)  Override, sys_judgement, Judgement
```

`fune get` が、新しい値とその範囲を表示するはずです。値の出どころも表示します。この場合は、部品の上書きと、その出典です。`fune unset OBC.mass` を実行すると上書きが消え、カタログの値 90 g に戻ります。

## 8. モードごとに部品の状態を決める

```console
$ fune mode set NOMINAL OBC on
$ fune mode set SAFE OBC on
$ fune mode table
 Part   NOMINAL   SAFE
───────────────────────
 OBC    on        on
```

`OBC` の 1 行だけの表が表示され、どちらのモードも `on` になるはずです。モード表はどのモードでどの部品がどの状態にあるかを示します。このモデルではバッテリーとパネルに状態がないので、行もありません。

## 9. 質量バジェットを読む

```console
$ fune budget mass
Mass budget
budget
     Row         Current   With growth   Allocated   Margin
────────────────────────────────────────────────────────────
 ✔   Power         430 g         461 g           —        —
 ✔   Structure     600 g         600 g           —        —
 ✔   C&DH         95.0 g        99.8 g           —        —
✔ Dry total 1.16 kg  limit 3.00 kg (system margin 10.0 %)  margin 1.54 kg
Propellant not counted: no propulsion (maneuvers or propulsion parts)
```

乾燥質量の合計は 1.16 kg で、限度 3.00 kg の下にあるはずです。列は公称値の現在の合計、成熟度ごとの成長余裕を加えた合計、サブシステムの配分（まだありません）、余裕です。末尾の行では合計を `R_MASS` の限度と、システム余裕 10 % に照らして比べています。これで最初の質量バジェットができました。

## 10. 電力の要求を追加して電力バジェットを読む

```console
$ fune requirement add R_POWER --statement "The orbit-average power consumption shall not exceed 5 W." \
    --quantity Power --kind at_most --value "5 W" --budget power --source sys_judgement
```

電力バジェットは一度に 1 つのモードを数えます。ケースがないと、`fune budget power` はモードを自分で選びます（このプロジェクトでは `SAFE` を選びました）。モードを指定するには、そのモードを指すケースを追加し、そのケースを既定にします。

```console
$ fune case set nominal_day --mode NOMINAL --label "Nominal day"
  created cases/nominal_day.toml
✔ Wrote 1 files  rev b3:2ede95432446
$ fune setting set analysis.common.default_case nominal_day
  created analysis/common.toml
✔ Wrote 1 files  rev b3:c493ce2fbd46
$ fune budget power --case nominal_day
Power budget
power.mode_load  case nominal_day  mode NOMINAL
     Row    Current   With growth   Allocated   Margin
───────────────────────────────────────────────────────
 ✔   C&DH    650 mW        683 mW           —        —
✔ Total 683 mW  limit 5.00 W (system margin 0 %)  margin 4.32 W
⚠ budget/allocation.toml has no [power]
```

ケースは解析の条件に名前を付けたひとまとまりです。ここでは条件はモードだけです。最後の行は電力をサブシステムに配分していないという注意です。

## 11. 解析を実行して不足している入力を埋める

電源系のエネルギーバランスを実行します。

```console
$ fune analyze power.energy_balance
✖ power.energy_balance: missing inputs (3)
 Missing input                           Reason
────────────────────────────────────────────────────────────────────────────────────────────────────────
 analysis.aocs.law.NOMINAL               Cannot pick the attitude law of the mode uniquely. Specify the attitude law of the target mode
 conditions.orbit                        Orbit settings (conditions/orbit.toml)
 requirement[power.energy_balance/dod]   Allowed DoD (an at_most requirement verified by power.energy_balance / dod)
To fix (replace the <…> parts):
  analysis.aocs.law.NOMINAL    fune set analysis.aocs.law.NOMINAL "<value>"
  conditions.orbit             fune set conditions.orbit "<value>"
```

解析は実行されず、値を推測もしませんでした。必要なものを知らせています。ここで必要なのは軌道、`NOMINAL` モードの姿勢則、限度です。終了コードは 1 なので、スクリプトでも検知できます。項目を 1 つずつ埋めます。

まず軌道です。パス `conditions.orbit.<キー>` は、`conditions/orbit.toml` のキーを 1 つ書きます。高度 550 km の円形の太陽同期軌道には次の値が要ります。

```console
$ fune set conditions.orbit.altitude "550 km"
$ fune set conditions.orbit.epoch "2027-04-01T00:00:00Z"
$ fune set conditions.orbit.eccentricity 0
$ fune set conditions.orbit.sun_synchronous true
$ fune set conditions.orbit.ltdn "10:30"
```

離心率には `0` と入力できます。`fune` はこの設定が実数だと知っているので、`conditions/orbit.toml` には `eccentricity = 0.0` と書きます。

次は姿勢則です。電源の解析ではパネルがどちらを向くかを知る必要があります。公称モードでは機体の軸の 1 つを太陽に向けるとします。キーの名前は、デモの `analysis/aocs.toml` と同じです。

```console
$ fune set analysis.aocs.law.NOMINAL.law sun_pointing
$ fune set analysis.aocs.law.NOMINAL.axis "1 0 0"
$ fune set analysis.aocs.law.NOMINAL.secondary "0 0 1"
```

最後は放電深度の限度です。限度は要求です。この要求は `power.energy_balance` の `dod` のチェックで確認するので、解析は限度をそこから読みます。

```console
$ fune requirement add R_DOD --statement "The battery depth of discharge shall not exceed 30 % in the nominal mode." \
    --quantity Fraction --kind at_most --value "30 %" --analysis power.energy_balance --check dod \
    --source sys_judgement --all-cases false
$ fune analyze power.energy_balance
✔ power.energy_balance ran
     KPI                                    Value
────────────────────────────────────────────────────
 ✖   Power balance (orbit average, worst)   -682 mW
 ✔   Max DoD                                 8.16 %
 Verdict       Target    Item                     Detail
───────────────────────────────────────────────────────────────────────────────────────────────────────
 ✔ ok          Nominal   DoD limit                Max DoD 8.16 % (limit 30.0 %, when sustained over the whole analysis span)
 ✖ violation   Nominal   Orbital energy balance   Energy margin per orbit -1.09 Wh (including battery efficiency, ...)
 ✔ ok          SAFE      DoD limit                Max DoD 8.16 % (limit 30.0 %, when sustained over the whole analysis span)
 ✖ violation   SAFE      Orbital energy balance   Energy margin per orbit -1.09 Wh (including battery efficiency, ...)
Use --json for the full result
```

解析が実行され、実際の判定が出ました。**violation**（違反）です。衛星は軌道ごとにエネルギーを失っています。理由は太陽電池パネルの法線が `0 0 1`（機体の +Z）なのに、姿勢則が機体の +X を太陽に向けるため、パネルが太陽に対して真横になっているからです。直すのは解析ではなく設計です。+Z を太陽に向けます。

```console
$ fune set analysis.aocs.law.NOMINAL.axis "0 0 1"
$ fune set analysis.aocs.law.NOMINAL.secondary "1 0 0"
$ fune analyze power.energy_balance --case nominal_day
✔ power.energy_balance ran
     KPI                                    Value
────────────────────────────────────────────────────
 ✔   Power balance (orbit average, worst)    13.9 W
 ✔   Max DoD                                0.979 %
 Verdict   Target    Item                     Detail
───────────────────────────────────────────────────────────────────────────────────────────────────────
 ✔ ok      Nominal   DoD limit                Max DoD 0.979 % (limit 30.0 %, when sustained over the whole analysis span)
 ✔ ok      Nominal   Orbital energy balance   Energy margin per orbit 22.1 Wh (including battery efficiency, ...)
Use --json for the full result
```

どちらの判定も `ok` になりました。`--case nominal_day` を付けると、解析はそのケースのモードだけを見ます。これで最初の解析の判定が得られました。

GUI では同じ不足している入力が Power ページの行として表示されます。各行には「Enter value」や「Add a part」などのボタンがあり、小さなダイアログが開きます。[不足している入力の行](gui.md#不足している入力の行と対処の操作)を参照してください。

## 12. 要求を確認する

```console
$ fune requirement compliance
 Verdict   ID        Requirement                                                                 Constraint   Cases     Detail
──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ✔         R_DOD     The battery depth of discharge shall not exceed 30 % in the nominal mode.     ≤ 30.0 %   default   DoD limit (Nominal): Max DoD 0.979 % (limit 30.0 %, ...)
 ✔         R_MASS    The satellite total mass shall not exceed 3 kg.                              ≤ 3.00 kg   default   Total 1.16 kg against limit 3.00 kg
 ✔         R_POWER   The orbit-average power consumption shall not exceed 5 W.                     ≤ 5.00 W   default   Total 683 mW against limit 5.00 W
3 requirements: 3 ok, 0 warning, 0 violation, 0 not verified
```

各要求の判定はその要求を確認するバジェットか解析のチェックから取られます。

## 13. 検査してコミットする

```console
$ fune check
✔ Check: no problems
$ fune fmt --check
$ git add -A
$ git commit -m "First satellite: bus, battery, solar panel, computer"
```

`fune fmt --check` は何も表示せず、終了コード 0 で終わります。`fune` が正規の形でファイルを書くので、すべてのファイルが正規の形です。ファイルを手で編集した場合は、このコマンドが異なるファイルを一覧にして、終了コード 1 で終わります。`fune fmt` で書き直せます。コミットの前と CI で、`fmt --check` と `check` を実行してください。

意図しない操作をしたときは、`fune undo` で直前の編集を取り消し、`fune redo` でやり直せます。`fune history` は編集の一覧を表示します（[CLI ガイド](cli.md#取り消しと履歴)）。

## できあがったもの

```text
my-sat/
├── fune.toml
├── analysis/
│   ├── aocs.toml            attitude law of the nominal mode
│   └── common.toml          the default case
├── budget/
│   ├── allocation.toml
│   └── policy.toml
├── cases/
│   └── nominal_day.toml
├── catalog/
│   ├── bat_2s.toml
│   ├── obc_a.toml
│   └── sp_body.toml
├── conditions/
│   └── orbit.toml
├── modes/
│   ├── NOMINAL.toml
│   └── SAFE.toml
├── parts/
│   ├── cdh/OBC.toml
│   ├── power/BAT.toml
│   ├── power/SP_PZ.toml
│   └── structure/BUS.toml
├── requirements/
│   ├── R_DOD.toml
│   ├── R_MASS.toml
│   └── R_POWER.toml
└── sources/
    ├── ds_vendor.toml
    └── sys_judgement.toml
```

（`.gitignore` と、git の対象外のキャッシュ `.fune/` は省略しています。）`fune` が書いたままのファイルを 2 つ示します。1 つ目はコンピューターで、カタログの項目を指し、値を 1 つ上書きする部品です。

```toml
# parts/cdh/OBC.toml
catalog = "local/obc_a"

[mount]
on = "BUS"

[pose]
position = { x = 0.0, y = 0.0, z = 10.0, unit = "mm" }

[attr.mass]
value       = 95.0
min         = 90.0
max         = 110.0
unit        = "g"
from.source = "sys_judgement"
from.note   = "Margin for the connector"
```

2 つ目はそれが指すカタログの項目です。

```toml
# catalog/obc_a.toml
kind     = "obc"
label    = "On-board computer"
maturity = "off_the_shelf"

[attr.mass]
value       = 90.0
unit        = "g"
from.source = "ds_vendor"

[attr.power]
off.value       = 0.0
off.unit        = "W"
off.from.source = "ds_vendor"
on.value        = 0.65
on.unit         = "W"
on.from.source  = "ds_vendor"
```

最後は解析の判定基準を持つ要求です。

```toml
# requirements/R_DOD.toml
statement = "The battery depth of discharge shall not exceed 30 % in the nominal mode."
quantity  = "Fraction"
source    = "sys_judgement"

[constraint]
kind  = "at_most"
value = { value = 30.0, unit = "%" }

[verify]
analysis = "power.energy_balance"
check    = "dod"
```

どのファイルも数行です。入力したコマンドと見比べてください。ファイルの形式は[データモデル](../../model.md)にまとめています。

## 次のステップ

- ほかのサブシステムを追加します。各[解析のページ](../analyses/index.md)に、その解析に必要な部品、設定、要求が書かれています。`fune analyze` のメッセージと `fune check` の警告が、次に足りないものを教えます。
- 用語を学びます。[概念](concepts.md)を読んでください。
- 完全なプロジェクトを見ます。[クイックスタート](quickstart.md)で使った、`fixtures/demo-6u` のデモです。
