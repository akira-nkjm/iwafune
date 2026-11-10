# iwafune 開発ルール

衛星の概念設計の検討基盤。設計データ（git で管理する TOML のディレクトリ）を CLI（`fune`）と GUI の両方から編集し、各系の解析を回す。
設計の全体は [docs/architecture.md](docs/architecture.md)、データの形は [docs/model.md](docs/model.md)、機能の置き場所は [docs/architecture/registry.md](docs/architecture/registry.md)、
今後の機能は [docs/roadmap.md](docs/roadmap.md) と GitHub の Issue。

## 構成のルール（最重要）

1. **機能の置き場所は構成台帳（registry.md）が正。** 新しい型・関数・モジュールを作る前に台帳を見る。同じ機能があればそれを使う。
   無ければ台帳に 1 行追記してから作り、§5 の変更履歴にも 1 行足す。台帳に無い場所に機能を足さない。
2. **1 つの機能は 1 か所だけ。** 積分器、天体の位置、食、座標変換、軌道伝搬、単位の解析・換算・表示、既定値、判定の基準、語彙の表示名、
   パスや ID の読み方などを 2 か所に書かない。似たものを見つけたら、作る前に相談する。
   同じ値を 2 つの crate が持つしかないとき（依存の向きで共有できないとき）は、両方が見える crate のテストで一致を確かめる。
3. **責務の違うものを 1 つにまとめない。** 計算と表示、読み込みと検証、Op の解釈とファイルの書き込みは別の関数・モジュールにする。
   目安は 1 ファイル 400 行・1 関数 60 行まで（係数表などのデータは除く）。
4. **依存の向きを守る。** `crates/lib/*` は `crates/app/*` に依存しない。crate の中のモジュール同士も循環させない（葉のモジュールに切り出す）。
5. **CLI と GUI の対等性。** すべての操作は `fune-service` の関数として作り、CLI と HTTP の両方から呼ぶ。人が打った文字列の解釈（`fune set` の値の型など）も
   service が持ち、GUI も同じ関数を通す（`POST /ops/from_text`）。CLI と HTTP で同じ編集をするとファイルがバイト単位で一致すること（パリティテスト）。
   例外は `fune init`（サーバーはプロジェクトの中でしか動かないので HTTP に無い）だけ。

`cargo xtask check` が機械的に検査する: 依存の向き、モジュールの循環、台帳、「1 か所だけ」の規則（`crates/xtask/src/single_home.rs`）、
Web の禁止事項（`fetch(` は API クライアントだけ、数の整形・`Intl.NumberFormat` は `web/src/quantity/` だけ、色の直書き（hex・`white` などの色の名前）と色の混ぜ方（`color-mix(`）は `lib/tone.ts` だけ（`index.css` の色は `pnpm -C web tokens` でそこから書き出す。`crates/xtask/src/web_colors.rs`）、
負の字間 `tracking-tight` を使わない、ID やパスを `/`・`.` で分けない、打った文字列を `Number(`・`parseFloat(` で数にしない（`crates/xtask/src/web_strings.rs`）、
スケルトン（`ui/skeleton`）は `kit/Loading.tsx` だけ、ページの URL（`/design/…` など）は `lib/nav.ts` の `paths` だけで作る、`features/` を import してよいのは
`features/` と `routes/` だけ（`crates/xtask/src/web_pages.rs`））、長すぎるファイルの警告。規則を増やしたら xtask にも足す。

## 層の責務

| crate | 責務 | してはいけないこと |
|---|---|---|
| `sat-units` | 単位・次元・量の唯一の正本。型付きの量（`Qty`）、表示の種類（`Fraction`・`Decibel*`）、量と単位の表示文字列（`format`） | — |
| `sat-astro` | 数学、積分器、時刻、軌道、天体、食、地球、重力、地磁気、大気、地上局、姿勢則、時系列。慣例の既定値（`with_defaults` など） | 解析固有の判断、ファイル I/O（大きなデータはバイト列で受け取る） |
| `sat-analysis` | 各系の解析と判定。入出力は型付きの SI 量。モデル化の既定値は解析が持つ（入力は `Option`） | `fune-*` への依存、I/O。判定文の数は `spec::quantity_text`（sat-units）だけで書く |
| `fune-model` | 設計データの型、組み込みの属性・種別・語彙の表示名、解決（Resolved）、検証（check） | ファイル I/O |
| `fune-store` | ディレクトリ ⇄ Design の読み書き（toml_edit、正規形 = `fune fmt`）、revision、ロック | 検証ロジック、物理計算 |
| `fune-ops` | Op（編集の唯一の入口）、plan / apply、to_cli | HTTP や表示 |
| `fune-projection` | Resolved → 解析の入力型への写し替えと、不足入力の列挙 | 物理計算、モデル化の既定値 |
| `fune-service` | CLI と HTTP が呼ぶ関数すべてと、その入出力の型（ts-rs で TS を生成）。表示用の文字列は `sat_units::format` で作って DTO に入れる | HTTP の知識、整形の規則を自前で持つこと。ファイル I/O は store を通す（外部データのキャッシュと TS の書き出しだけは例外） |
| `fune-server` | axum のルーティングと JSON 化、SPA の配信、Host の許可リスト | ロジック |
| `fune-cli` | 引数の解析と表示（表・色・診断） | ロジック |
| `web/` | 表示と操作 | 物理計算、次元の計算、単位の換算、文字列の分解による ID やパスの解釈（API が構造で返す） |

## コーディング規約

- Rust: edition 2024、stable の最新。`cargo fmt` と `cargo clippy --all-targets -- -D warnings` を通す。
- エラーはライブラリでは `thiserror`、バイナリでは `anyhow`。ライブラリのコードで `unwrap` / `expect` を使わない（テストは可）。
- 公開する型・関数には `///` で説明を書く（日本語）。各 crate の lib.rs の先頭に、持っているモジュールと責務を書く。
- 解析の関数には手計算や文献値と比べるテストを最低 1 つ付ける。リファクタリングではデモの解析結果の数値が変わらないことを確かめる。
- TypeScript: strict、Biome、`any` を使わない。API の型は `web/src/api/gen/`（ts-rs の生成物）だけを使い、手で書かない。
  型を変えたら `cargo test -p fune-service export_ts` で作り直してコミットする（CI が差分を検査する）。
- Web の UI: 文言は i18n の辞書（日本語と英語の両方）。数と量の表示は API の整形（`Formatted`）を使う。読み込み中は `kit/Loading.tsx` のスケルトン。
  書体は Noto Sans JP（等幅は Noto Sans Mono と BIZ UDゴシック）。Radix の `asChild` に渡す子の `className` は文字列にする（関数は崩れる）。
- 依存は最新の安定版を固定する（2 週間以上前に出た版）。パッケージ管理は pnpm。言語は Rust と TS だけ。
- TOML の書き方は docs/model.md §10（書き出しは TOML 1.0 の範囲）。デモのデータ（`fixtures/`）も CLI（`fune part place`・`fune set` など）で編集し、
  `fune fmt --check` を通す。デモのデータは架空のもので、実在の部品の値や配布に制限のある資料の値は入れない。

## 進め方

- コミットメッセージは英語。日々の作業は `develop` に入れる（CI: 構成・Rust・web・同梱ビルド）。
- 作業は Issue に紐づける。Issue ごとに develop からブランチ `issue-<番号>-<短い名前>`（例: `issue-21-port-refs`）を切り、
  develop への PR で入れる。PR の本文には、Issue を閉じるなら `Closes #<番号>`、段階の一部なら `Part of #<番号>` を書く。
  コミットメッセージの件名の末尾にも `(#<番号>)` を付ける。Issue の無い小さな直し（文書の誤字など）だけは develop に直接入れてよい。
- `main` はリリースだけ。develop からの PR でだけ入れ、PR の前に develop で版を上げる（`Cargo.toml` の `[workspace.package] version` と
  `web/package.json` の `version` を同じ値に）。PR では release.yml が「版が揃っていて、まだ出ていない」ことを検査し、main に入ると
  コンテナイメージ（`ghcr.io/<owner>/fune:<版>`）・タグ `v<版>`・GitHub Release を作る。タグは手で打たない。
- 検査が全部通ってからコミットする。

## よく使うコマンド

```bash
cargo xtask check                 # 依存・循環・台帳・「1 か所だけ」の検査
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo test -p fune-service export_ts   # TS の型を作り直す
cargo run -p fune-cli -- -C fixtures/demo-6u check
cargo run -p fune-cli -- -C fixtures/demo-6u fmt --check
pnpm -C web lint && pnpm -C web typecheck && pnpm -C web test && pnpm -C web build
```
