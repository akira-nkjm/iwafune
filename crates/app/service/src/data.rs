//! 外部データ（DE440 の暦表・宇宙天気の表）の取得・状態・読み込み。**ファイルを読むのはここで、sat-astro はバイト列を受け取るだけ。**
//!
//! 置き場所はユーザーごとのキャッシュ: `$IWAFUNE_DATA_DIR`、無ければ `$XDG_CACHE_HOME/iwafune/data`、無ければ `~/.cache/iwafune/data`。
//! git にもリリースの成果物にも入れない。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `release` | リリース配布のデータの共通の仕組み（表の 1 行 = [`Dataset`]。取得・SHA-256 の照合・状態・登録）。データを足すときは表に 1 行と登録する関数を足す |
//! | `de440` | 表の最初の行（DE440 の暦表）と、sat-astro への登録 |
//! | `radiation` | 放射線帯の地図（AP-8・AE-8）・古い地磁気モデルの係数・IGRF の行と、sat-astro への登録。群の ID `radiation` で 7 件をまとめて扱う |
//! | [`space_weather`] | 宇宙天気（CelesTrak SW-All）。毎日更新されるので取得のたびに取得日時と SHA-256 を記録し、どの版を使ったかを返す |
//! | `verify` | ファイルの印（大きさ・更新時刻）が変わらない間、求めた結果を使い回す覚え |

mod de440;
mod radiation;
mod release;
pub mod space_weather;
mod verify;

use release::{Dataset, Downloader};

use crate::dto::{DataItem, DataStatus};
use crate::error::{Result, ServiceError};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// キャッシュのディレクトリ。
pub fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("IWAFUNE_DATA_DIR").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(xdg).join("iwafune/data"));
    }
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(|h| PathBuf::from(h).join(".cache/iwafune/data"))
        .ok_or_else(|| {
            ServiceError::Data(
                "Cannot decide the cache location (neither HOME nor XDG_CACHE_HOME is set)".into(),
            )
        })
}

/// リリース配布のデータの表。**データを足すときはここに 1 行足す**（状態・取得・起動時の登録はこの表で決まる）。
static RELEASE_DATASETS: [&Dataset; 8] = [
    &de440::DE440,
    &radiation::AP8_MIN,
    &radiation::AP8_MAX,
    &radiation::AE8_MIN,
    &radiation::AE8_MAX,
    &radiation::JENSEN_CAIN,
    &radiation::GSFC_1966,
    &radiation::IGRF14,
];

/// データの状態（`GET /data/status`、`fune data status`）。ファイルの SHA-256 も確かめる
/// （同じプロセスの中では、ファイルの大きさと更新時刻が変わらない間は前回の結果を使う）。
pub fn data_status() -> Result<DataStatus> {
    let dir = cache_dir()?;
    Ok(DataStatus {
        cache_dir: dir.display().to_string(),
        ephemeris: ephemeris_source(),
        items: all_items(&dir, &RELEASE_DATASETS)?,
    })
}

/// 表のデータの状態に、宇宙天気（毎日更新される取得物で、表の形に乗らない）を続けたもの。
fn all_items(dir: &Path, table: &[&Dataset]) -> Result<Vec<DataItem>> {
    let mut items = table
        .iter()
        .map(|d| d.item(dir))
        .collect::<Result<Vec<_>>>()?;
    items.push(space_weather::item(dir)?);
    Ok(items)
}

/// キャッシュにあるリリース配布のデータを読み込んで登録する（プロセスで一度だけ。CLI と HTTP サーバーの起動時に明示的に呼ぶ）。
pub fn install_cached_datasets() {
    if let Ok(dir) = cache_dir() {
        for dataset in RELEASE_DATASETS {
            dataset.install_cached(&dir);
        }
    }
}

/// データを取得してキャッシュに置く（`POST /data/{id}/fetch`、`fune data fetch <id>`）。
/// 一時ディレクトリに落として確かめてから置き換えるので、失敗しても既存のファイルは壊れない。
///
/// `from` があれば、ネットワークの代わりにそのディレクトリにあるファイル（リリースの成果物と同じ名前）を取り込む
/// （配布が済んでいないデータや、ネットワークの無い環境のため。**CLI だけの選択肢**で、HTTP は渡さない: サーバーのマシンの上の場所を HTTP から指させない。大きさと SHA-256 は取得したときと同じに確かめる。
/// 宇宙天気は取り込めない）。`id` に群の ID `radiation` を与えると、放射線帯のデータ 7 件をまとめて扱う。
pub fn fetch(id: &str, from: Option<&Path>) -> Result<DataStatus> {
    if from.is_some() && id == space_weather::ID {
        return Err(ServiceError::Data(
            "space_weather is fetched from the network and cannot be installed from a directory"
                .into(),
        ));
    }
    let dir = cache_dir()?;
    let ids: Vec<&str> = if id == radiation::GROUP_ID {
        radiation::GROUP_IDS.to_vec()
    } else {
        vec![id]
    };
    for one in ids {
        match from {
            Some(source) => fetch_in(&dir, one, &RELEASE_DATASETS, &copy_from(source))?,
            None => fetch_in(&dir, one, &RELEASE_DATASETS, &release::gh_download)?,
        }
    }
    data_status()
}

/// ディレクトリ `source` にある成果物を一時ディレクトリへ写す取得関数（`fetch` の `from`）。
fn copy_from(source: &Path) -> impl Fn(&release::ReleaseSpec, &Path) -> Result<()> + '_ {
    move |spec, tmp| {
        let from = source.join(spec.file);
        std::fs::copy(&from, tmp.join(spec.file))
            .map(|_| ())
            .map_err(|e| io_error(&from, e))
    }
}

/// 放射線帯の解析が要る地図と場のうち、読み込めていないものの名前（そろっていれば空）。
pub fn radiation_missing() -> Vec<String> {
    sat_astro::radiation::RadiationTables::installed().missing()
}

/// `fetch` の本体。表と取得の関数を引数に取る（試験では差し替える）。
fn fetch_in(dir: &Path, id: &str, table: &[&Dataset], download: Downloader<'_>) -> Result<()> {
    type Run<'a> = Box<dyn Fn(&Path, &Path) -> Result<()> + 'a>;
    let run: Run<'_> = if let Some(dataset) = table.iter().find(|d| d.spec.id == id) {
        Box::new(move |tmp, dir| dataset.fetch(tmp, dir, download))
    } else if id == space_weather::ID {
        Box::new(space_weather::fetch)
    } else {
        return Err(ServiceError::not_found("data", id));
    };
    std::fs::create_dir_all(dir).map_err(|e| io_error(dir, e))?;
    let tmp = dir.join(format!(".download-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| io_error(&tmp, e))?;
    let result = run(&tmp, dir);
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

pub(crate) fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|e| io_error(path, e))?;
    Ok(sha256_hex(&bytes))
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn io_error(path: &Path, e: std::io::Error) -> ServiceError {
    ServiceError::Data(format!("{}: {e}", path.display()))
}

/// 運用期間（2020 年代）の元期で、解析が使う暦の種類。
fn ephemeris_source() -> sat_astro::ephem::EphemerisSource {
    sat_astro::ephem::source_at(sat_astro::time::Epoch::from_calendar(2026, 1, 1, 0, 0, 0.0))
}

#[cfg(test)]
mod tests;
