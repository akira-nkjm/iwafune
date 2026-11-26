# インストール

このページでは `fune` のインストール、動作確認、更新、任意のデータセットの取得、削除の方法を説明します。`fune` は 1 つのプログラムです。コマンドラインツールと Web の GUI を含んでいるので、ほかにインストールするものはありません。

## 方法を選ぶ

| 方法 | 向いている場合 |
|---|---|
| [リリースのバイナリ](#リリースのバイナリ) | いちばん簡単に入れたい場合。Linux（x86_64）と macOS |
| [cargo-binstall](#cargo-binstall) | すでに Rust のツールを使っていて、1 つのコマンドで済ませたい場合 |
| [コンテナイメージ](#コンテナイメージ) | CI のジョブで `fune check` だけを実行したい場合 |
| [ソースから](#ソースから) | お使いのプラットフォーム向けのリリースバイナリがない場合や、iwafune 自体を開発する場合 |

リリースはすべて、リポジトリの GitHub Releases ページで公開されています。各リリースには次のファイルが付いています。

| ファイル | 対象 |
|---|---|
| `fune-<version>-x86_64-linux-gnu.tar.gz` | glibc 2.39 以降の x86_64 Linux（Ubuntu 24.04 以降。WSL を含む） |
| `fune-<version>-universal-apple-darwin.tar.gz` | Apple Silicon と Intel の macOS |

各アーカイブには対応する `.sha256` ファイルがあります。

## リリースのバイナリ

アーカイブは GitHub CLI（`gh`）か Releases ページからダウンロードします。チェックサムを確認して展開し、`PATH` の通ったディレクトリに `fune` を置きます。次のコマンドは Linux 向けです。

```bash
gh release download --repo akira-nkjm/iwafune --pattern 'fune-*-x86_64-linux-gnu.tar.gz*'
sha256sum -c fune-*-x86_64-linux-gnu.tar.gz.sha256
tar -xzf fune-*-x86_64-linux-gnu.tar.gz
install -m 755 fune-*-x86_64-linux-gnu/fune ~/.local/bin/fune
```

macOS では `x86_64-linux-gnu` の代わりに `universal-apple-darwin` を使い、ファイルの確認には `shasum -a 256 -c` を使います。

macOS のバイナリは公証されていません。`gh` でダウンロードしたファイルはそのまま動きます。ブラウザでダウンロードしたファイルは隔離属性を外すまで Gatekeeper に止められます。

```bash
xattr -d com.apple.quarantine fune
```

そのあとは[インストールの確認](#インストールの確認)に進みます。

## cargo-binstall

[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) は、ビルドせずにリリースのバイナリをダウンロードします。

```bash
cargo binstall --git https://github.com/akira-nkjm/iwafune fune-cli
```

そのあとは[インストールの確認](#インストールの確認)に進みます。あとで更新するときは `fune self-update` を実行します（[更新](#更新)を参照）。

## コンテナイメージ

リリースごとに、CLI のコンテナイメージ `ghcr.io/akira-nkjm/fune:<version>`（と `:latest`）も公開されます。イメージには `git` が含まれています。CI で設計をチェックする用途を想定しています。

別のリポジトリがこのイメージを取得できるようにするには、パッケージの「Manage Actions access」にそのリポジトリを追加します。ワークフローはそのリポジトリ自身の `GITHUB_TOKEN` で、イメージをジョブのコンテナとして使います。

```yaml
jobs:
  check:
    runs-on: ubuntu-latest
    container: ghcr.io/akira-nkjm/fune:latest
    permissions:
      contents: read
      packages: read
    steps:
      - uses: actions/checkout@v4
      - run: fune fmt --check
      - run: fune check
```

共有カタログを使う場合に必要な追加の手順は [CLI ガイド](cli.md#共有カタログ)にあります。

## ソースから

お使いのプラットフォーム向けのリリースバイナリがないときに使います。Rust 1.100 以降、Node.js 24 以降、pnpm が必要です。`fune` のバイナリはビルド済みの GUI ファイルを配信するので、先に GUI をビルドします。

```bash
pnpm -C web install && pnpm -C web build
cargo run -p fune-cli -- -C fixtures/demo-6u part ls
```

2 つ目のコマンドは `fune` をビルドし、デモのプロジェクトの部品を一覧にします。部品の表が表示されるはずです。開発の全体の流れ、リポジトリの構成、リリースの手順は、[開発](../../development.md)にあります。

## インストールの確認

```console
$ fune --version
fune 0.10.0
$ fune --help
The `fune` command-line interface: argument parsing and output only (tables, colors, diagnostics)

Usage: fune [OPTIONS] <COMMAND>

Commands:
  init           Create a new project
  check          Check the design data (exit code 1 on errors)
  gui            Start the GUI (a local HTTP server) and open it in the browser
  ...
```

バージョン行とコマンドの一覧が表示されるはずです。バージョン番号はお使いのものと異なる場合があります。シェルが `command not found` と表示するときは、`fune` を置いたディレクトリが `PATH` に入っていません。

設計に対して試すには、[クイックスタート](quickstart.md)に進みます。

## 更新

`fune self-update` は実行中のバイナリを最新のリリースで置き換えます。まず、何が行われるかを確かめます。

```console
$ fune self-update --check
Checking the latest release...
✔ fune 0.10.0 is already the latest version
```

新しいリリースがあるときは、出力にそう表示されます。更新するには `fune self-update` を実行します。古い版も含めて特定の版を入れるには、`fune self-update --version <major.minor.patch>` を使います。

`self-update` は何かを置き換える前に、`curl` でアーカイブをダウンロードし、`tar` で展開し、SHA-256 を検証し、新しいバイナリが期待する版を報告することを確かめます。どれかの手順に失敗したときは、何も置き換えません。`fune` を置いたディレクトリへの書き込み権限が必要です。バイナリの入れ方（リリースのアーカイブ、cargo-binstall、ソースからのビルド）によらず使えます。

`fune self-update` が変更するのはプログラム自体です。プロジェクトには触れません。

## 任意のデータセット

一部の解析は同梱するには大きすぎるデータを使います。データを取得するまではこれらの解析は不足している入力として報告し、実行されません。キャッシュにあるデータは次のコマンドで確かめます。

```console
$ fune data status
 ID                 State               Size      Loaded   Location
─────────────────────────────────────────────────────────────────────────────────────────────
 de440              ✔ SHA-256 matches   20.7 MB   yes      ~/.cache/iwafune/data/de440.bin
 ap8min             ✔ SHA-256 matches    102 kB   yes      ~/.cache/iwafune/data/ap8min.asc
 ap8max             ✔ SHA-256 matches    101 kB   yes      ~/.cache/iwafune/data/ap8max.asc
 ae8min             ✔ SHA-256 matches   81.3 kB   yes      ~/.cache/iwafune/data/ae8min.asc
 ae8max             ✔ SHA-256 matches   83.6 kB   yes      ~/.cache/iwafune/data/ae8max.asc
 jensen_cain_1960   ✔ SHA-256 matches   2.27 kB   yes      ~/.cache/iwafune/data/jensen_cain_62.dat
 gsfc_1966          ✔ SHA-256 matches   36.6 kB   yes      ~/.cache/iwafune/data/gsfc66.dat
 igrf14             ✔ SHA-256 matches   42.4 kB   yes      ~/.cache/iwafune/data/igrf14coeffs.txt
 shieldose2         ✔ SHA-256 matches    712 kB   yes      ~/.cache/iwafune/data/shieldose2.dat
 space_weather      not fetched               —   —        ~/.cache/iwafune/data/SW-All.csv
Fetch: fune data fetch space_weather
```

上の表は放射線データと天体暦がすでにインストールされているマシンのものです。新しくインストールした環境ではほとんどの行が `not fetched` になります。ここでは `Location` 列を `~/…` に短くしています。お使いのマシンの表示は実際のキャッシュのパスです。

各データセットの用途は次のとおりです。

| ID | 使う解析 |
|---|---|
| `de440` | 太陽、月、惑星の位置。なければ解析式を使います。推進解析の日付つきの遷移でも使います。 |
| `space_weather` | 大気モデルの太陽フラックス F10.7 と地磁気指数 Ap（日付ごとの抗力による減衰）。 |
| `ap8min`, `ap8max`, `ae8min`, `ae8max`, `jensen_cain_1960`, `gsfc_1966`, `igrf14`, `shieldose2` | 捕捉放射線の解析。`orbit.trapped_flux`、`orbit.trapped_dose`、`orbit.total_dose` が計算する環境。1 つのコマンドでグループ全体がインストールされます。 |

必要なデータを取得します。次のコマンドはインターネットからダウンロードし、書き込み先はプロジェクトではなくユーザーのキャッシュです。

```bash
fune data fetch de440           # GitHub CLI（gh）が必要
fune data fetch space_weather   # curl が必要
fune data fetch radiation       # gh が必要
```

放射線のファイルがすでにあるディレクトリから、代わりにインストールすることもできます。

```bash
fune data fetch radiation --from <DIR>
```

`fune data fetch` は `--from` のときも含めて、サイズと SHA-256 を確認します。取得したあと、`fune data status` ではその行が `✔ SHA-256 matches` になります。放射線データの出典と帰属は [捕捉放射線のデータ](../../trapped-radiation.md)にあります。データは GUI からも取得できます。[Settings](gui.md#settings) を参照してください。

## アンインストール

バイナリを削除します。データのキャッシュが不要なら、それも削除します。

```bash
which fune                 # バイナリの場所を表示する
rm "$(which fune)"
rm -r ~/.cache/iwafune     # データのキャッシュ。パスは `fune data status` の Location 列に出る
```

プロジェクトは普通のディレクトリです。その外に片づけるものはありません。エージェントスキル（`fune skill install`）を使った場合は、プロジェクトの `.claude/skills/fune/` も削除してください。
