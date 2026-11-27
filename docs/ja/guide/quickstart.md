# クイックスタート

このツアーは 10 分ほどです。デモ衛星をコピーし、チェックし、GUI で見て、値を 1 つ変え、バジェットが赤くなるのを確かめ、解析を実行し、変更を git で読み、元に戻します。`fune`（[インストール](install.md)）と git が必要です。

各ステップに、することと、表示されるはずの結果を書いています。`…` だけの行は出力を省略したことを示します。デモのデータは固定なので、数値も同じになります。

## 1. デモを入手する

デモはリポジトリの `fixtures/demo-6u` にある架空の 6U 衛星です。自由に編集できるようにコピーし、あとで変更を確かめられるように、コピーを git の管理下に置きます。

```bash
git clone https://github.com/akira-nkjm/iwafune
cp -r iwafune/fixtures/demo-6u my-demo
cd my-demo
git init -b main
git add -A
git commit -m "Baseline demo"
```

`my-demo` というディレクトリができ、コミットが 1 つあるはずです。これがプロジェクトで、`fune.toml` を含むディレクトリです。`fune` はカレントディレクトリとその上のディレクトリから、このファイルを探してプロジェクトを見つけます。

## 2. チェックする

```console
$ fune status
iwafune-6u-demo
/path/to/my-demo/.  branch main  rev b3:5e54b14f2
parts 38  groups 11  modes 4  nets 8  harnesses 2  cases 3
✔ Check: no problems
$ fune check
✔ Check: no problems
```

どちらのコマンドも、`✔ Check: no problems` で終わるはずです。

`fune status` はプロジェクトが持つものを数えます。`fune check` は単位と次元、存在しない ID への参照、必須の値の不足を検証します。エラーがあると終了コード 1 で終わるため、スクリプトや CI でも使えます。`rev` は設計ファイルのハッシュで、どれかのファイルが変わると変わります。

## 3. GUI を開く

```console
$ fune gui
✔ Started the GUI: http://localhost:3000/
Project: /path/to/my-demo  (Ctrl-C to quit)
```

ブラウザが開き、Overview ページが表示されます。ポート 3000 が使用中のときは、`fune gui` が次の空きポートを使い、その番号を表示します。`--no-open` でブラウザを開かないようにでき、`--port` でポートを選べます。コマンドは実行したままにして、以下のコマンドは 2 つ目の端末で実行します。終わったら、1 つ目の端末で Ctrl-C を押して止めます。

<!-- screenshot: / as budgets — デモの Overview ページ。KPI カード、Mass budget と Power budget のパネル、Diagnostics パネル -->
*デモのプロジェクトの Overview ページ。*

## 4. Overview ページを読む

Overview ページはダッシュボードです。上から順に、次のものがあります。

- KPI カード（たとえば "Mass margin"）。各カードは値、バー、判定の色を示します。カードをクリックすると、その元になっているサブシステムのページが開きます。
- バジェットのパネル（たとえば "Mass budget"）。各行はサブシステムで、現在値、成長余裕を含めた値、配分、余裕を示します。赤い行は配分を超えています。
- Diagnostics。`fune check` と同じ件数（エラー、警告、情報）を示します。

質量バジェットの Payload の行を探してください。現在値 920 g、成長余裕込み 1.10 kg、配分 1.20 kg、余裕 96.0 g です。次に、この値を変えます。

左のサイドバーにはすべてのページが並んでいます。ページごとの説明は [GUI ガイド](gui.md)にあります。

## 5. 値を 1 つ変える

カメラ `CAM` の質量は 920 g です。レンズ鏡筒が重く、1.2 kg だと分かったとします。

GUI の場合は、サイドバーの Components を開き、`CAM` を選びます（ツリーか表でクリックするか、`Ctrl+K` で検索します）。詳細パネルで Mass の行を探し、その値をクリックします。`1.2 kg` と入力して Enter を押すと、値はすぐに保存されます。サイドバー下部の "Change history" に、この変更が、同じ編集を行うコマンドとともに表示されます。

```text
fune set CAM.mass "1.2 kg"
```

コマンドラインの場合は、同じ編集を、出典とメモを付けて行います。先に `--dry-run` を付けると、何も書き込まずにファイルの差分を確かめられます。

```console
$ fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel" --dry-run
--- a/parts/payload/CAM.toml
+++ b/parts/payload/CAM.toml
@@ -8,6 +8,12 @@
 [pose]
 position = { x = 0.0, y = -60.0, z = -90.0, unit = "mm" }

+[attr.mass]
+value       = 1.2
+unit        = "kg"
+from.source = "em_test_2026_10"
+from.note   = "Heavier lens barrel"
+
 [ports.PWR]
 net = "P5V"

✔ dry-run: 1 files would change (nothing written)
$ fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"
  modified parts/payload/CAM.toml
✔ Wrote 1 files  rev b3:3f80e3738632
```

どちらか一方だけを行ってください。GUI を使った場合、ファイルには `value` と `unit` の行はありますが、`from.source` と `from.note` の行はありません。この値はカタログの値 920 g を上書きします。`em_test_2026_10` はデモにある出典です（`fune source ls` で一覧できます）。

## 6. バジェットと判定の変化を見る

```console
$ fune budget mass
Mass budget
budget
     Row             Current   With growth   Allocated   Margin
────────────────────────────────────────────────────────────────
 ✔   AOCS            1.04 kg       1.10 kg     1.60 kg    502 g
 ✔   C&DH             90.0 g        99.0 g       200 g    101 g
 ✔   Communication     340 g         370 g       500 g    130 g
 ✖   Payload         1.20 kg       1.44 kg     1.20 kg   -240 g
 ✔   Power           1.45 kg       1.58 kg     2.00 kg    423 g
…
```

変更前の Payload の行は 920 g、1.10 kg、1.20 kg、96.0 g でした。いまは ✖ が付いています。成長余裕を含めると 1.44 kg が必要で、配分は 1.20 kg だからです。GUI の Overview ページを開くと、同じ行が赤くなっています。合計は限度内のままです。衛星全体には余裕があり、サブシステムごとの配分は合計とは別に判定するためです。

<!-- screenshot: / as mass-over — CAM.mass を 1.2 kg にしたあとの Mass budget パネル。Payload の行が配分を超えて赤くなっている -->
*変更後に配分を超えた Payload の行。*

## 7. 解析を実行する

解析は ID を指定して実行します。`fune analyze ls` ですべての解析を一覧できます。ここでは、構造の質量特性の解析を実行します。

```console
$ fune analyze structure.mass_properties
✔ structure.mass_properties ran
     KPI          Value
──────────────────────────
 ✔   Total mass   5.83 kg
 Verdict   Target   Item         Detail
───────────────────────────────────────────────────────────────────────────────────────────────
 ✔ ok               Total mass   With growth margin 5.83 kg / usable 12.0 kg (limit 12.0 kg × (1 − system margin 0 %))
Use --json for the full result
```

解析は KPI（主要な数値）と判定を出力します。判定は `ok`、`warning`、`violation` のいずれかです。ここでは、総質量が限度の 12 kg を大きく下回っています。この限度は要求 `R_MASS` によるものです。変更前、この解析は 5.55 kg でした。

コマンドごとに数値が違うのは数えているものが違うからです。上の解析が報告するのは成長余裕を含めた総質量です。ステップ 6 の質量バジェットは推薬も加えるため、合計は 6.64 kg です。

適合表はすべての要求を、それを検証する解析かバジェットと並べて示します。ここでは R_MASS の行と最後の行だけを残しています。

```console
$ fune requirement compliance
 Verdict          ID               Requirement                                       Constraint   Cases     Detail
…
 ✔                R_MASS           The satellite total mass shall not exceed 12 kg.   ≤ 12.0 kg   default   Total 6.64 kg against limit 12.0 kg
…
16 requirements: 11 ok, 1 warning, 1 violation, 3 not verified
```

デモにはすでに違反が 1 件（`R_STORAGE`）と警告が 1 件（`R_STT_SUN`）あるので、判定を読む練習ができます。各判定の意味は[解析](../analyses/index.md)を参照してください。

同じ解析を GUI で実行するには、サブシステムのページ（たとえば Structure）を開きます。Overview タブに KPI と判定が表示され、"Analysis results" タブと "Settings" タブでさらに詳しく見られます。

## 8. 変更を git で読む

行ったことはすべて 1 つのファイルの変更です。

```console
$ git status --short
 M parts/payload/CAM.toml
$ git diff
diff --git a/parts/payload/CAM.toml b/parts/payload/CAM.toml
index 5d01e09..0001e51 100644
--- a/parts/payload/CAM.toml
+++ b/parts/payload/CAM.toml
@@ -8,6 +8,12 @@ point = "CAM_PORT"
 [pose]
 position = { x = 0.0, y = -60.0, z = -90.0, unit = "mm" }

+[attr.mass]
+value       = 1.2
+unit        = "kg"
+from.source = "em_test_2026_10"
+from.note   = "Heavier lens barrel"
+
 [ports.PWR]
 net = "P5V"
```

これが iwafune の考え方の中心で、設計の変更は読める差分になります。コミットも、プルリクエストも、破棄もできます。

## 9. 元に戻す

`fune history` は元に戻せる編集を一覧にします。`fune undo` は直前の編集が変えたファイルを元に戻します。

```console
$ fune history
   1          fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"  1 file(s)
(keeps the last 100 edits; `fune undo` / `fune redo` move along this list)
$ fune undo
  modified parts/payload/CAM.toml
✔ Undid: fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"  rev b3:5e54b14f210a
$ git status --short
```

`git status` は何も表示しません。作業ツリーはベースラインに戻り、`rev` もステップ 2 のもの（`b3:5e54b14f2…`）になっています。`fune redo` で変更をやり直せます。GUI では `Ctrl+Z`（Mac では `⌘Z`）で元に戻し、`Ctrl+Y`（`⇧⌘Z`）でやり直せます。GUI と CLI は 1 つの履歴を共有しており、サイドバー下部の "Change history" には、このセッションの編集が表示されます。

編集のあとでファイルがほかの方法（エディタや `git checkout`）で変更されていると、元に戻す操作は実行されません。自分が行っていない作業を上書きしないためです。

## 次に読むもの

- 何もない状態から自分のプロジェクトを作る: [最初の衛星を作る](first-satellite.md)
- いま見たものの背後にあるモデルを学ぶ: [概念](concepts.md)
- 作業ごとのコマンドを探す: [CLI ガイド](cli.md)。GUI のページを探す: [GUI ガイド](gui.md)
