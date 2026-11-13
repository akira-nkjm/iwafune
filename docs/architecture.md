# iwafune のアーキテクチャ

今のコードがどう組み立てられているかを書く。
設計データの形とファイル形式は [model.md](model.md)、機能ごとの置き場所は [architecture/registry.md](architecture/registry.md)（構成台帳）、
開発の規則は [CONTRIBUTING.md](../CONTRIBUTING.md)、これから入れる機能は [roadmap.md](roadmap.md) と GitHub の Issue にある。

前提条件（軌道・地上局・打ち上げ条件）は `conditions/`。`mission/` は将来の #9 用の予約ディレクトリで現在は読まない。
解析の所属は `sat_analysis::spec::AnalysisArea`（系 `System` または前提条件の話題 `Condition`）。
話題の設定は条件のファイル内に置き、具体的な型は #3 で追加する（[model.md §1.1](model.md#11-前提条件と解析の区分)）。
CLI・API・web は同じ区分を使い、KPI は系・話題ごとに表示する。web の前提条件の KPI は空なら表示しない。

## 1. 目的と範囲

iwafune は小型衛星の概念設計のための道具である。部品の選定、質量・電力・データ量のバジェット、モードごとの消費電力、
電源・AOCS・C&DH・通信・熱・構造の解析を、1 つの設計データの上で扱う。

- 設計データの正本は git で管理する TOML のディレクトリだけ。常駐のサーバーもデータベースも持たない。
- 編集の手段は CLI（`fune`）と Web の GUI の 2 つで、できることは同じ。GUI は `fune gui` が手元で起動するサーバーから配信する。
- 解析は概念設計の精度で、呼ぶたびに計算し直す。結果は設計データに書かない。
- 飛行ソフトウェア、テレメトリ・コマンドの設計、運用は扱わない。

## 2. 設計データ

### 2.1 ディレクトリ

衛星 1 機の設計は 1 つのディレクトリで、ルートに `fune.toml` を置く。`fune` は今いる場所から上へ `fune.toml` を探す（`-C` で場所を指定できる）。
1 エンティティを 1 ファイルにし、ファイル名を ID にする。

| 場所 | 中身 |
|---|---|
| `fune.toml` | プロジェクト名、スキーマの版、使うカタログ |
| `parts/` | 部品。サブディレクトリで製品ツリーを表す |
| `catalog/` | カタログ品（購入品などの値と出典） |
| `defs/attribute/`・`defs/kind/` | プロジェクト独自の属性定義・種別定義（組み込みの定義は fune-model が持つ） |
| `modes/`・`nets/`・`harness/` | 運用モード（部品 → 状態の表）、電気的なネット、ハーネスの区間 |
| `budget/` | 配分とマージンの方針 |
| `conditions/`・`analysis/`・`cases/` | 軌道と地上局、系ごとの解析の設定、解析ケース |
| `sources/` | 出典の台帳 |
| `view/` | 系統図の配置など、見た目だけの情報 |
| `.fune/` | 書き込みロックなどの作業ファイル。`fune init` が `.gitignore` に入れる |

ファイルの書き方、値と単位の表し方、エンティティの関係は [model.md](model.md) にある。
デモの設計データは [fixtures/demo-6u](../fixtures/demo-6u)。

### 2.2 ファイルにする理由

- PR の差分がそのまま設計の変更として読める。値を 1 つ変えると、変わるのはその行だけ（§2.4）。
- 検討をブランチで分けられる。1 エンティティ 1 ファイルなので、別のブランチで別の部品を触っても衝突しない。
- エディタで直接書き換えてもよい。`fune fmt` が正規形に揃え、`fune check` が検証する。

### 2.3 読み込みと解決

| 層 | 中身 | 作るところ |
|---|---|---|
| Design | 書かれたままの値（数値と単位の文字列）、出典、ファイル上の位置 | `fune_store::load` |
| Resolved | カタログ・種別・部品での上書きを重ねた値（SI）と、その出所 | `fune_model::resolve` |
| 解析の入力 | 解析ごとの型付きの SI 量 | `fune_projection`（§6.3） |

検証は `fune_model::check` の 1 か所で、`fune check`・GUI・編集の前後のどれも同じ関数を通る。
診断には重さ（error・warning・info）が付く。error は単位や次元の誤り、存在しない ID の参照、取付の循環など。
必須の値が欠けているのは warning なので、作りかけの設計も保存できる。

### 2.4 書き込み

ファイルの読み書きは fune-store だけが行う。

- toml_edit で、変えた値の行だけを書き換える。コメント、キーの順序、書かれた単位（`620 g` は `g` のまま）は残す。
- 新しいファイルと `fune fmt` は正規形（TOML 1.0 の範囲、決まったキーの順序、`=` の揃え）で書く。
- 複数のファイルにまたがる変更は、一時ファイルに書いてから rename で置き換える。途中で失敗したら、置き換えた分を戻してからエラーを返す。
- revision は追跡しているファイル（パスと本文）の BLAKE3。読んだときの revision と今の revision が違えば書かない（楽観ロック）。
- ほかのプロセスとの同時書き込みは `.fune/lock` で防ぐ。

### 2.5 ブランチと PR でのレビュー

fune は git の操作をしない。GUI に出すブランチ名を `.git/HEAD` から読むだけで、コミット・push・PR は利用者が行う。

1. 検討ごとにブランチを切る（`git switch -c study/rw-4wheel`）。
2. CLI か GUI で編集する。GUI のサイドバーの下には、ブランチ、revision、診断の件数と、このセッションで行った編集の一覧が出る。
   一覧の各行は CLI のコマンドとしてコピーできる。
3. コミットして PR を出す。差分には変えた値の行だけが出る。
4. 設計データのリポジトリの CI で `fune check` と `fune fmt --check` を流す。リリースごとに出すコンテナイメージ（§8）を
   ジョブのコンテナにすれば、ビルドせずに `fune` を使える。手順は [README](../README.md) にある。

実際の設計データは衛星ごとの private リポジトリに置く。このリポジトリにはデモの設計データしか入れない。
NDA や輸出管理の対象になる値を、公開するリポジトリに入れない。

## 3. crate と依存の向き

![crate の構成](diagrams/crate-architecture.drawio.svg)

図の元は [diagrams/crate-architecture.drawio](diagrams/crate-architecture.drawio)（draw.io で開ける）。

`sat-*` は `crates/lib/`、`fune-*` は `crates/app/` にある（`sat-units` なら `crates/lib/units`）。
「依存してよい」の列は `crates/xtask` の `ALLOWED` と同じで、外れると `cargo xtask check` が失敗する。

| crate | 依存してよい | 責務 | してはいけないこと |
|---|---|---|---|
| `sat-units` | なし | 単位・次元・量の唯一の正本。型付きの量（`Qty`）、表示の種類（`Fraction`・`Decibel*`）、量と単位の表示文字列（`format`） | — |
| `sat-astro` | units | 数学、積分器、時刻、軌道、天体、食、地球、重力、地磁気、大気、地上局、姿勢則、時系列。慣例の既定値（`with_defaults` など） | 解析固有の判断、ファイル I/O（大きなデータはバイト列で受け取る） |
| `sat-analysis` | units・astro | 各系の解析と判定。入出力は型付きの SI 量。モデル化の既定値は解析が持つ（入力は `Option`） | `fune-*` への依存、I/O |
| `fune-model` | units | 設計データの型、組み込みの属性・種別・語彙の表示名、解決（Resolved）、検証（check） | ファイル I/O |
| `fune-store` | model | ディレクトリ ⇄ Design の読み書き（toml_edit、正規形）、revision、ロック | 検証、物理計算 |
| `fune-ops` | model・store | Op（編集の唯一の入口）、plan / apply、Op → CLI のコマンド（`to_cli`） | HTTP、表示 |
| `fune-projection` | model・units・astro・analysis | Resolved → 解析の入力型への写し替えと、不足入力の列挙 | 物理計算、モデル化の既定値 |
| `fune-service` | 上のすべて | CLI と HTTP が呼ぶ関数すべてと、その入出力の型（ts-rs で TS を生成）。表示用の文字列は `sat_units::format` で作る | HTTP の知識、整形の規則を自前で持つこと |
| `fune-server` | service | axum のルーティングと JSON 化、SPA の配信、Host の許可リスト | ロジック |
| `fune-cli` | service・server | `fune` の引数の解析と表示（表・色・診断）。`fune gui` のために server を使う | ロジック |
| `web/` | HTTP API だけ | GUI の表示と操作 | 物理計算、次元の計算、単位の換算、文字列の分解による ID やパスの解釈 |

- `crates/lib/*` は物理と解析のライブラリで、設計モデルを知らない。`crates/app/*` に依存しない。
- server と cli は service だけを見る。model や ops の型は service が再公開したものを使う。
- projection はファイルを読まない（store にも ops にも依存しない）。
- crate の中のモジュールも循環させない。

## 4. 編集: Op と plan / apply

### 4.1 Op

設計データを変える操作は、すべて `fune_ops::Op` で表す。CLI のコマンドも GUI の操作も Op にしてから適用する。
Op は JSON にできる（`{ "op": "set_value", "part": "RW_X", ... }`）。TS の型も生成する。

| 対象 | Op の例 |
|---|---|
| 部品 | `CreatePart`・`RemovePart`・`RenamePart`・`RenameGroup`・`MovePart`・`SetValue`・`UnsetValue`・`SetField`・`Place`・`SetPort`・`SetMountPoint` |
| カタログ品 | `CreateCatalogItem`・`SetCatalogValue`・`UnsetCatalogValue`・`SetCatalogField`・`RenameCatalogItem`・`RemoveCatalogItem` |
| 定義 | `CreateAttribute`・`CreateKind` |
| モード・ネット | `CreateMode`・`RenameMode`・`SetModeStates`・`CreateNet`・`UpdateNet`・`SetNetField`・`RemoveNet` など |
| ケース・出典・ハーネス | `SetCase`・`SetSource`・`SetHarness` と、それぞれの削除 |
| 図 | `SetViewLayout`・`SetViewEdge`・`UnsetViewEdge`・`ResetView` |
| 設定 | `SetSetting`（`conditions/`・`analysis/`・`budget/` の値） |

すべての Op とその意味は `crates/app/ops/src/op.rs` にある。

### 4.2 plan と apply

- `plan` は書き込まない。Op を 1 つずつ store の編集に変え、そのたびに Design を読み直す。
  最後に適用の前と後で `check` を比べ、新しく増えた error があれば拒む。元からある error は編集を止めない。
- `apply` はロックを取り（ほかのプロセスが持っていれば最大 5 秒待つ）、読み込み、revision を確かめ、plan を作り、まとめて書く。
  全部が通るか、何も書かないかのどちらかになる。
- `dry_run` なら plan だけを返す。CLI の `--dry-run` と GUI の確認の画面は、どちらも変わるファイルの unified diff を見せる。

### 4.3 CLI と HTTP の対等性

CLI と HTTP は `fune-service` の同じ関数を呼ぶ。

| 操作 | CLI | HTTP（`/api/v1`） | service の関数 |
|---|---|---|---|
| Op の適用 | 編集の各コマンド、`fune apply ops.json` | `POST /ops` | `edit::apply` |
| 打った値の文字列 → Op | `fune set`・`fune setting set`・`fune catalog set` | `POST /ops/from_text` | `edit::set_op` |
| 消す先のパス → Op | `fune unset`・`fune setting unset`・`fune catalog unset` | `POST /ops/unset_from_text` | `edit::unset_op` |
| 名詞のコマンドに打った引数 → Op | `fune net set|unset`・`fune part place`・`fune mode set|unset`・`fune attr add`・`fune kind add` | `POST /ops/from_command` | `edit::command_ops` |
| 整形 | `fune fmt [--check]` | `POST /fmt` | `format::format_design` |

- `fune set RW_X.mass "620 g"` の `"620 g"` を量・整数・ベクトル・状態ごとの値のどれとして読むかは、属性定義を見て service が決める。
  GUI も打った文字列を `/ops/from_text` で Op にしてから `/ops` で送るので、解釈が 2 つに分かれない（部品・カタログ品の値の欄、
  設定の表、部品の基本の欄、パラメーター追加のダイアログのどれも）。同じ編集で定義を作る属性の値は、依頼の `ops`（下書き）を当てた設計で読む。
  GUI が送るパスは API が返したもの（`PartRow.paths`・`field_paths`、同じ編集で定義を作る属性は `POST /parts/{id}` の `ops` を当てた部品の `paths`）で、
  Web は ID やパスの文字列をつながない。
- パスと値の 1 組でない編集（ネットのレール電圧、部品の配置の位置と姿勢、モード表の時間割合、属性・種別の定義の作成）は、
  CLI の名詞のコマンドと同じ引数の形（`CommandText`。`{ command: "part_place", part, on, point, position: "0 0 12 mm", … }`）で
  打った文字列のまま `/ops/from_command` に送る。量・ベクトル・クォータニオン・割合・カンマ区切りの一覧・`名前=種類` を読むのも、
  表示名の既定（空なら ID）を決めるのも、CLI と同じ service の関数（`edit::{command, defs}`）。Web は打った値を数にしない
  （`cargo xtask check` が `Number(`・`parseFloat(` を検査する）。
- 適用の結果には、同じ編集をする CLI のコマンド（`fune_ops::to_cli`）が入る。GUI の「CLI としてコピー」はこれを使う。
- `fune apply` は GUI が送るのと同じ Op の JSON を受ける。スクリプトからの一括編集にも使える。
- 書き込まずに編集を当ててみることもできる（`fune_service::preview`）。`POST /analyses/{id}`・`POST /budgets/{kind}`・`POST /parts/{id}` の `ops`、
  `fune analyze --ops`・`fune budget --ops`・`fune part show --ops` が、保存前の下書き（モード表の編集など）のまま解析・バジェット・部品の表示を作る。
- 例外は `fune init` だけ。サーバーはプロジェクトの中でしか動かないので、HTTP には無い。

テストで次の 2 つを確かめる。

- 同じ編集を `fune` の実行ファイルと HTTP で行うと、ファイルがバイト単位で一致する（`crates/app/cli/tests/parity.rs`・`parity_text.rs`・`parity_command.rs`）。
- すべての種類の Op で、`to_cli` の文字列を clap で読み直すと元の Op に戻る（`crates/app/cli/src/cmd/roundtrip/`）。
  Op の種類を足すと、見本を足すまでコンパイルが通らない。

### 4.4 エラーと終了コード

| 分類 | 例 | HTTP | CLI の終了コード |
|---|---|---|---|
| 検証 | 編集で新しい error が出る | 422 | 1 |
| 入力の誤り | 引数・単位・Op の形 | 400 | 2 |
| 不在 | 部品や解析が無い | 404 | 2 |
| 競合 | revision が違う、ロックが取れない、同じ ID がもうある | 409 | 3 |
| その他 | I/O など | 500 | 1 |

HTTP の失敗は `{ "error": { "code", "message" }, "diagnostics": [...] }` で返し、CLI も `--json` で同じ形を出す。
保存しない下書き（解析・バジェット・部品の表示の `ops`）が通らないときも、保存と同じ分類で返す（検証なら 422 と診断）。
入力が足りないのは誤りではないので、解析もバジェットも 200 で `missing`（足りない入力の対象と理由）を返す。
`fune check` は error があれば 1 で終わる（`--deny-warnings` なら warning でも）。

## 5. サーバーと Web アプリ

### 5.1 fune-server

`fune gui` が同じバイナリの中でサーバーを起動し、ブラウザを開く。

- 既定の待ち受けは `127.0.0.1:8787`。`--port`・`--host`・`--no-open` で変えられる。認証は無い。
- API は `/api/v1` の下にある。ハンドラーは引数を読んで service を呼び、JSON にするだけ。ファイル I/O と解析は `spawn_blocking` で別のスレッドに回す。
- Host の許可リスト: API は設計ファイルを書けるので、Host ヘッダーが許可リストに無いリクエストは 403（`host_not_allowed`）で拒む（DNS リバインディング対策）。
  許可するのは `localhost`・`127.0.0.1`・`[::1]`（ポートは問わない）、`--host` に指定した IP（`0.0.0.0` は除く）、`--allow-host` で足した名前。
  Host ヘッダーの無いリクエスト（ブラウザ以外のクライアント）は通す。
- CORS: origin がローカルホストのリクエストだけを許す（GET・POST と `Content-Type`）。開発中の Vite（別のポート）から呼ぶため。
- SPA の同梱: `web/dist` を rust-embed でバイナリに埋め込む。クライアント側のパス（`/design/components/RW_X` など）には `index.html` を返し、
  `assets/` のファイルは長くキャッシュさせる。`web/dist` が無いままビルドすると、GUI のビルドの手順を書いたページを返す。
- サーバーは設計を覚えておかず、リクエストのたびにディレクトリから読み直す（数十ファイルで数 ms）。CLI で書き換えても、次のリクエストから GUI に出る。
  ファイルの監視はしていないので、GUI はウィンドウに戻ったときと自分の編集のあとに読み直す。

| パス | 内容 |
|---|---|
| `GET /project`・`/design`・`/parts`・`/parts/{id}`・`/values/{path}`・`/defs`・`/diagnostics` | 読み取り（概要と revision、設計全体、部品、1 つの値、定義、診断） |
| `POST /ops`・`/ops/from_text`・`/ops/unset_from_text`・`/ops/from_command`・`/fmt` | 編集（§4.3） |
| `POST /parts/{id}` | 保存していない編集（`ops`）を書き込まずに当てた部品（`GET` と同じ形。`fune part show --ops`） |
| `POST /parts/{id}/place_body` | 機体座標での配置を、取付先に対する相対の `Place` に直して適用する |
| `GET /units`・`POST /units/parse`・`/units/format` | 単位の一覧、入力の解析（次元と SI 換算）、SI 値の表示文字列 |
| `GET /analyses`・`POST /analyses/{id}`・`GET`/`POST /budgets/{kind}` | 解析の一覧と実行、バジェット（§6。POST の `case`・`ops` で解析ケースと下書きを指定） |
| `GET /data/status`・`POST /data/{id}/fetch` | 外部データの状態と取得（§6.5） |

### 5.2 Web アプリ

`web/` は React 19 の SPA（React Router 8 の SPA モード、Vite でビルド）。サーバーの状態は TanStack Query で持つ。
画面の部品は Tailwind CSS 4 と Radix、系統図は @xyflow/react（配置と配線は `features/diagram/` の自前の自動配置）、
3D 配置は three・@react-three/fiber・@react-three/drei、グラフは Recharts。lint と整形は Biome、テストは Vitest。

画面は概要（バジェットと診断）、部品、カタログ、系統図（給電・信号）、モード表、配置（左の部品のツリー・中央の 3D・右の配置の欄。部品の選択・取付・位置を画面全体の 3D で確認・編集し、質量特性があれば重心を表示）、定義（属性・種別・単位）、
系ごとのページ（電源・AOCS・C&DH・通信・熱・構造・ミッション機器。ミッション機器の解析は未登録。構造の概要から配置ページへ移動できる。温度で塗った 3D は熱のページに残す）、軌道と地上局、解析ケース、診断、設定。
系のページはその系に関係する部品をその系の属性の列だけで並べてセルで編集でき、同じページに解析の結果を出す。
系のタブは「概要（KPI と判定）・モード・部品・時系列・設定」（英語: Overview, Modes, Parts, Time series, Settings）。
モードのタブは電源・C&DH・熱・AOCS に置く。電源は消費電力の行列と成長余裕込みの合計・使用可能電力・余裕・判定、
C&DH は圧縮・パケット化前のデータ発生レート、熱は節点に属する部品の内部発熱、AOCS はモードごとのカードに主軸・副軸の図と姿勢則、役割別の機器・稼働台数・状態、最長継続時間を表示する。
AOCS カードの運動量蓄積・アンロード頻度・MTQ アンロード時間・センサ視野判定は Rust で評価する。
`fune_projection::aocs::{rw_unloading_for_mode,sensor_fov_for_mode}` にモードを明示し、
既定ケースの軌道条件（β）を保って姿勢則だけを選ぶ。通常の解析のケース選択・入力・数値は変えない。
機器は日照・蝕を切り替えられ、集計は `counted_states` を共用し、姿勢則・軸の編集は設定と同じ SetText の入口を通す。
日照／蝕の値・合計・判定は `system_modes::system_modes`（`POST /system_modes/{system}`）が既存の射影・解析から返す。
電源の使用可能電力はバジェットの配分上限からシステム余裕を引いた設計値で、日照・蝕とも同じ上限を使う（軌道上の発電や電池の瞬時能力とは別）。
設計の「モード」は定義を編集する唯一の場所（追加・名前・継続時間・部品の状態・稼働台数・予備・蝕の上書き）。
状態の選択の参考に単体の電力レベル列は残すが、セル内の解析値・合計・収支は各系のタブに置く。
各系のセルの状態変更も同じ `draft.ts`・`useModeDraftLoad`・`fune mode set|unset` の引数 → Op → 保存を使う。
系統図は API の `DefsView.systems` の系ごとに表示を切り替え、既定で構造を隠す。
選択は `lib/nav.ts` を通じて URL の `hidden-system` に保存する（繰り返し可、空は全表示）。
隠したカードに接する線と空の枠は描かず、残る線をつなぎ替えない。隠れた供給元・マスターの線も隠す。
隠した組立品の可視の子は可視の祖先に置く。表示の変更は設計データを変更しない。
URL は `src/routes.ts`、サイドバーの並びは `src/lib/nav.ts` にある。

Web は次のことをしない。

- 物理計算、次元の計算、単位の換算。量の入力は `POST /units/parse` で解析し、表示にはサーバーが `sat_units::format` で作った文字列（`Formatted`）を使う。
- 文字列を分解して ID やパスを読むこと。API が構造で返す。
- API の型を手で書くこと。型は ts-rs が Rust から書き出した `src/api/gen/` だけを使う（`cargo test -p fune-service export_ts` で作り直し、CI が差分を検査する）。

次の機能は置き場所を 1 つに決めている（xtask が検査する）。

| 機能 | 場所 |
|---|---|
| HTTP の呼び出し（`fetch(`） | `src/api/client.ts` |
| 問い合わせの無効化（`invalidateQueries`） | `src/api/`（キーと、編集のあとに読み直す範囲は `keys.ts`） |
| 数の整形（`Intl.NumberFormat`・`toFixed` など） | `src/quantity/` |
| 色の値 | `src/lib/tone.ts`（`index.css` の色は `pnpm -C web tokens` で書き出す。テストがコントラストを確かめる） |

文言は日本語と英語の辞書（`src/i18n/`）に置く。2 つの辞書の項目が揃っていることはテストで確かめる。
開発中は `pnpm -C web dev`（5173 番）が `/api` を `fune gui --no-open` の `127.0.0.1:8787` に中継する（`FUNE_API` で変えられる）。

## 6. 解析

### 6.1 一覧

| 系 | 定常 | 時系列 |
|---|---|---|
| 電源 | `power.mode_load`（モード別消費電力）、`power.solar_sunlit`（日照時の発電） | `power.orbit_generation`（軌道上の発電）、`power.energy_balance`（電力収支と DoD） |
| AOCS | `aocs.unload_time`（MTQ・RCS でのアンロードにかかる時間） | `aocs.rw_unloading`（擾乱トルクと RW のアンロード頻度）、`aocs.sensor_fov`（センサの FoV と禁止角） |
| C&DH | `cdh.data_budget`（データ量とストレージ） | — |
| 通信 | `comm.link_budget`（回線バジェット） | `comm.visibility`（地上局の可視）、`comm.data_throughput`（データ送信量と蓄積） |
| 熱 | `thermal.steady`（多節点の定常温度） | `thermal.transient`（多節点の非定常温度） |
| 構造 | `structure.mass_properties`（質量特性）、`structure.static_load`（準静的荷重）、`structure.modal_estimate`（簡易固有振動数。FEM はしない） | — |

`fune analyze ls` と `GET /analyses` が同じ一覧を返す。

### 6.2 解析の形

すべての解析は `sat_analysis::spec::Analysis` を実装する。

- ID（`<系>.<名前>`）、題名と説明（日英）、系、定常か時系列か、`Input`・`Output` の型と、`run(&Input) -> Result<Output>` を持つ。
- `run` は純粋な関数で、I/O も乱数も使わない。設計モデル（`fune-*`）を知らない。
- 入出力は sat-units の型付きの量（`Qty`）で、値は SI。割合は `Fraction` にする。
- 解析の登録は `sat_analysis::catalog` の 1 か所だけ。ID と JSON の入力で実行する `run_json` もここにある。
- モデル化の既定値は解析が持ち、入力では `Option` にする。軌道要素や地上局の慣例の値は sat-astro の `with_defaults` が持つ。
- 時系列の解析は軌道と姿勢則から時系列（時刻・位置・速度・太陽方向・日陰・機体姿勢）を `sat_astro::timeline` で作り、その上で計算する。
  制御のループを回すシミュレーションはしない。

### 6.3 入力の組み立てと結果

解析は次の流れで実行する（`fune_service::analysis::run_analysis`。`fune analyze <ID>` と `POST /analyses/{id}` が呼ぶ）。

1. 設計を読み、Resolved を作る（`ops` があれば書き込まずに当てる）。
2. fune-projection が解析ケース（`cases/<ID>.toml`。省略すると既定）を含む文脈から、その解析の `Input` を組み立てる。
   解析 ID と射影の対応は `fune_projection::dispatch` の 1 か所にある。
3. 足りない入力があれば解析を呼ばずに `missing_inputs` を返す。`missing_inputs` は足りないものをパラメーターのパス（`RW_X.max_momentum`・`conditions.orbit.altitude`）と理由で表し、
   見つかった分をすべて並べる。
4. `sat_analysis::catalog::run_json` で解析を実行する。
5. service が出力の量ごとの次元（JSON Pointer → 次元名）と語彙の値（列挙）の表示名を集め、時系列に表示名と表示単位を付け、
   出力の中の判定を並べて、最も悪い判定を付ける。解析は表示名を持たず、キー（`ChannelKind`・列挙の値・`span_fallback`）だけを返す
   （日本語と英語の表示名は `fune_service::analysis::labels`）。

入力の与え方は依頼の `source` で選ぶ。`design`（既定）は設計から組み立て、`patched`（JSON Merge Patch）は組み立てた入力の一部を一時的に上書きし、
`given`（CLI の `--input`）は入力を丸ごと与える。

判定は解析が出す。`Check` は key・label・判定（`ok`・`warning`・`violation`）・文を持つ。
余裕の三段階の判定は `spec::margin_verdict` で決め（警告の幅は解析ごとに名前付きの定数）、判定文の中の数は `spec::quantity_text::<次元の型>`（中身は sat-units の整形）で書く。
比の余裕の定義は `spec::ratio_margin`（`能力 / 要求 − 1`）の 1 つだけ。
バジェット（質量・電力・データ量）の集計と判定は `sat_analysis::budget::budget_rollup` が行う。
service は部品や解析の結果を項目に並べ、`budget/allocation.toml` の配分の行に割り当てるだけ。成長余裕は部品の成熟度と `budget/policy.toml` から決める。
バジェットも解析ケースを指定できる（`POST /budgets/{kind}` の `case`、`fune budget --case`）。電力はケースのモードで、データ量はケースのモードだけで数える。
系に属さず部品への明示配分も無い部品は「その他」の行にまとめる（行の `node` が null。文言は CLI と Web が作る）。

### 6.4 sat-astro

解析が使う物理と数学の部品は sat-astro にあり、どれも 1 か所だけに置く。

| モジュール | 中身 |
|---|---|
| `ode` | 固定刻みの RK4 と、適応刻みの DOP853（Hairer の `dop853.f` の移植、密出力付き）。積分器はこれだけ |
| `orbit` | 軌道要素、伝搬（Kepler・J2 の永年項・力モデルの数値積分）、太陽同期の傾斜角、β 角。中心天体は地球・月・太陽・惑星 |
| `ephem` | 太陽・月・惑星の位置。解析式（太陽は VSOP87A の切り詰め、月は Meeus、惑星は Standish の表）と、登録すれば JPL DE440 |
| `force` | 重力場（EGM96、70 次まで。係数は埋め込み）、第三体、太陽輻射圧、大気抵抗、固体潮汐、相対論の補正 |
| `shadow` | 影（円柱・円錐）。食の判定はこれだけ |
| `earth`・`time` | ECI ⇄ ECEF、測地座標、歳差、元期と恒星時 |
| `geomag`・`atmosphere` | 地磁気（傾斜双極子）、大気密度（指数大気・NRLMSISE-00） |
| `ground` | 地上局の見通し角、パスの検出、地上軌跡 |
| `attitude`・`illumination`・`timeline` | 姿勢則（太陽指向・地心指向・慣性固定）、面への太陽入射、軌道上の時系列 |

入力の仕様（`OrbitSpec`・`GroundStation` など）は型付きの量で受け、内部の関数は f64 の SI 値で受け渡す。
設計データから解析に渡す軌道の伝搬は、二体＋J2 の永年項（既定）か二体だけ。

### 6.5 外部データ（DE440）

DE440 の暦表（`de440.bin`、約 20.7 MB。JPL DE440 を sat-astro が読む形式に変換したもの）は git に入れず、このリポジトリのリリース（タグ `data-de440-v1`）に置く。

- 取得は `fune data fetch de440`（GUI では設定のページ、HTTP では `POST /data/de440/fetch`）。中で `gh release download` を呼ぶので、GitHub CLI が要る。
  一時ディレクトリに落とし、SHA-256 を確かめてから置き換えるので、失敗しても今あるファイルはそのまま残る。
- 置き場所はユーザーごとのキャッシュ。`$IWAFUNE_DATA_DIR`、無ければ `$XDG_CACHE_HOME/iwafune/data`、無ければ `~/.cache/iwafune/data`。
- CLI とサーバーは起動時に一度だけキャッシュを読み、sat-astro に登録する。ファイルを読むのは `fune_service::data` で、sat-astro はバイト列を受け取るだけ。
- 登録すると 1849〜2150 年は DE440 を使い、無ければ解析式を使う。今どちらを使うかは `fune data status`・`GET /data/status` で分かる。

## 7. 機械的な検査

`cargo xtask check`（`crates/xtask`）が構成の規則を検査し、違反があれば失敗する。

| 検査 | 中身 |
|---|---|
| 依存の向き | ワークスペースの crate が、§3 の表で許した crate にだけ依存しているか |
| モジュールの循環 | crate の中のモジュール同士が循環していないか |
| 台帳 | すべての crate が [registry.md](architecture/registry.md) に載っているか |
| 1 か所だけ | 台帳で置き場所を決めた定義（積分器、μ や地球半径などの定数、影、座標変換、単位の解析、解析の登録など）が、ほかのファイルに無いか（`crates/xtask/src/single_home.rs`） |
| Web の規則 | `fetch(`・数の整形・問い合わせの無効化・色の直書き・負の字間（`tracking-tight`）が、決めた場所の外に無いか |
| 言語 | Python のファイルが無いか（言語は Rust と TypeScript だけ） |
| ファイルの長さ | 400 行を超えるファイルを警告する（係数表などのデータは除く） |

規則を増やしたら、xtask にも足す。

CI（`.github/workflows/ci.yml`）は main・develop への push と PR で動く。

| ジョブ | 中身 |
|---|---|
| architecture | `cargo xtask check` |
| rust | `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --workspace`（パリティテストを含む）、生成した TS の型が Rust と一致するか |
| web | Biome、型検査、Vitest、ビルド |
| bundle | SPA を同梱したリリースビルドを作り、デモの設計データで CLI と HTTP を試す |

## 8. ブランチとリリース

- 日々の作業は `develop` に入れる。`main` には `develop` からの PR だけを入れ、`main` に入るものはすべてリリースになる。
- PR を出す前に `develop` で版を上げる。`Cargo.toml` の `[workspace.package] version` と `web/package.json` の `version` を同じ値にする。
- `release.yml`
  - main への PR: 2 つの版が一致していて、そのタグがまだ無いことを確かめる。
  - main への push: `image.yml` を呼んでコンテナイメージを出し、macOS のバイナリを作ってから、タグ `v<版>` と GitHub Release（変更点は自動生成）を作る。
    Release には、Linux と macOS のバイナリの tar.gz とその SHA-256 を添付する。リリース済みのコミットで動き直しても何もしない。
  - macOS のバイナリ: Apple Silicon の runner（macos-15）で arm64 をビルドし、x86_64 をクロスコンパイルして、`lipo` で 1 つにまとめる
    （`fune-<版>-universal-apple-darwin.tar.gz`）。GUI は `image.yml` がビルドしたものを使う。公証はしていない。
- `image.yml`: SPA をビルドし、ubuntu-24.04 で `fune` をビルドして（イメージの元と glibc を揃える）、`fune --version` が版と合うかを確かめる。
  バイナリが同梱した GUI を返すことを確かめてから、`fune-<版>-x86_64-linux-gnu.tar.gz`（バイナリ・README・ライセンス）にまとめる。
  `docker/fune.Dockerfile` でイメージを作り、デモの設計データで `fune check` と `fune fmt --check` を通してから、
  `ghcr.io/<owner>/fune:<版>` と `:latest` を push する。
- タグは手で打たない。

## 9. これから

予定している機能は [roadmap.md](roadmap.md) と GitHub の Issue にある。この文書には今あるものだけを書き、機能が入ったら書き足す。

部品の最上位の区分は系（`parts/<系 ID>/`）。`Design::system_of` が置き場所 → 役割 → 未所属の順で決め、Resolved と API の `PartRow.system` に渡す。系とバジェットの表示名は組み込みの語彙（日英）、色は Web の `lib/systems.ts` の対応だけが持つ。配分のキーは系か部品の ID で、上限は任意（未設定を check が警告）。init は上限を決めない。

3D は `features/layout3d/scene.ts` が回転込みの外形（Bounds）の最大辺から表示寸法を作る。仮の箱・方向の矢印・重心球・座標軸・グリッド・カメラは `SCENE_RATIOS` の比を共有する。形状も位置の広がりも無いときだけ表示用の基準を使う。編集のスナップはユーザーが選ぶ実寸、操作ハンドルと画面隅のギズモは画面に対する大きさ。

配置のページは CAD のブラウザーのように、左に部品のページと同じ製品ツリー（`PartsTree`。`parts/` のフォルダーと組立品の親子）を出す。ツリーで部品を押すと 3D でも選び、視点をその部品の外形に寄せる（向きは保つ）。3D で選んだ部品はツリーでも選ばれ、閉じたフォルダーは開く。部品とフォルダーの目のアイコンで 3D の箱を出し入れし（フォルダーは子孫をまとめて）、操作の帯で原点の軸・地面の格子・画面隅の軸を出し入れする。どれもページの中の状態で、設計データにも URL にも書かない（表示寸法は隠した部品も含めた外形のまま）。幅が狭いと 3D・ツリー・配置の欄の順に縦に並び、中くらいの幅ではツリーと 3D を横に、配置の欄をその下に置く。
