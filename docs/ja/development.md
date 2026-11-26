# iwafune の開発

このページは iwafune 自体をビルドしたり変更したりする人向けです。使い方は [README](../../README.md) を見てください。
貢献のルール（構造、コーディング規約、ワークフロー）は [CONTRIBUTING.md](../../CONTRIBUTING.md) にあり、このページでは繰り返しません。

## ソースからのビルド

必要なもの: Rust 1.100 以降（edition 2024）、Node.js 24 以降、pnpm。

```bash
pnpm -C web install && pnpm -C web build                       # build the GUI (the fune binary embeds web/dist when it is compiled)
cargo run -p fune-cli -- -C fixtures/demo-6u gui                # open the GUI on the demo 6U satellite
cargo run -p fune-cli -- -C fixtures/demo-6u part ls            # list parts from the CLI
cargo run -p fune-cli -- -C fixtures/demo-6u analyze power.energy_balance
```

## リポジトリの構成

| パス | 内容 |
|---|---|
| `crates/lib/units` (`sat-units`) | 単位、次元、型付きの量 |
| `crates/lib/astro` (`sat-astro`) | 数学、積分器、時刻、軌道、天体暦、食、地球のモデル、姿勢則、放射線帯と遮蔽 |
| `crates/lib/checkkey` (`sat-checkkey`) | 解析の判定のキーの書式。解析とモデルが共有する |
| `crates/lib/analysis` (`sat-analysis`) | 型付きの SI 入出力を持つサブシステム解析。設計モデルを知らない |
| `crates/app/model` (`fune-model`) | 設計データの型、組み込みの語彙、解決と検証 |
| `crates/app/store` (`fune-store`) | TOML ディレクトリの読み書き、リビジョン、ロック |
| `crates/app/ops` (`fune-ops`) | 編集の唯一の入口である `Op`。計画、適用、CLI コマンドへの変換 |
| `crates/app/projection` (`fune-projection`) | 解決した設計から解析の入力への変換と、不足入力の一覧 |
| `crates/app/service` (`fune-service`) | CLI と HTTP API が呼ぶすべての関数と、その型（TypeScript に書き出す） |
| `crates/app/server` (`fune-server`) | HTTP API（axum）と組み込みの GUI |
| `crates/app/report` (`fune-report`) | レポートの書き出し。表と図のデータモデル、SVG のグラフ、Typst モジュール |
| `crates/app/cli` (`fune-cli`) | `fune` コマンド |
| `crates/xtask` (`xtask`) | `cargo xtask check`。アーキテクチャのルール |
| `web/` | GUI（React、TypeScript） |
| `fixtures/demo-6u` | デモの 6U 衛星 |
| `docs/` | ユーザーガイド（`guide/`）、解析（`analyses/`）、データモデル、アーキテクチャとレジストリ、レポートの書き出し、捕捉放射線のデータ、ロードマップ。日本語版は `ja/` |

`crates/lib/*` は `crates/app/*` に依存しません。`cargo xtask check` は、依存の向きと、[architecture/registry.md](architecture/registry.md) に記録した「1 つの機能は 1 か所に置く」ルールを検査します。
編集はすべて `Op` で、CLI と HTTP API は同じサービス関数を呼びます。同じ編集をどちらで行っても、バイト単位で同一のファイルが書かれることをパリティテストが確かめます。全体の設計は [architecture.md](architecture.md) にあります。

## 開発用コマンド

```bash
cargo xtask check                              # dependency and registry rules
cargo test --workspace
cargo clippy --all-targets -- -D warnings
pnpm -C web lint && pnpm -C web typecheck && pnpm -C web test
cargo test -p fune-service export_ts           # regenerate web/src/api/gen after changing API types
```

CI はアーキテクチャの検査、Rust の検査（フォーマット、clippy、パリティテストと生成した型の検査を含むテスト）、Web の検査（lint、型検査、テスト、ビルド）を実行します。`develop` と `main` への push では、さらにリリースバイナリをビルドし、CLI と HTTP サーバーをスモークテストで確かめます。

## ブランチとリリース

日々の作業は `develop` に入れます。`main` は `develop` からのプルリクエストだけを受け取り、1 つ 1 つがリリースになります。
先に `develop` でバージョンを上げます（`Cargo.toml` の `[workspace.package] version` と `web/package.json` の `version` を同じ値にします）。
[release.yml](../../.github/workflows/release.yml) は、プルリクエストでバージョンが整合していて未リリースであることを確かめます。マージ後にコンテナイメージを公開し（[image.yml](../../.github/workflows/image.yml)）、タグ `vX.Y.Z` とその GitHub Release を作ります。タグは手で push しません。
