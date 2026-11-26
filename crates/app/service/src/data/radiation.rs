//! 放射線帯の束縛粒子の地図（AP-8・AE-8）と、地図が前提にする古い地磁気モデルの係数、IGRF の配布ファイルの行と、
//! sat-astro（`sat_astro::radiation::registry`）への登録。**ファイルの意味・形式は sat-astro が持ち、ここはバイト列を渡すだけ。**
//!
//! 配布の元: NASA の NSSDC データセット PT-11B（NASA CCMC の ModelWeb のアーカイブにあるものを無変更で再配布する）と IAGA の IGRF。
//! 出所の表示（`license`）は表の行ごとに持ち、`fune data status` と GUI の設定画面が出す。

use super::release::{Dataset, ReleaseSpec};
use sat_astro::geomag::{GaussModel, IgrfTable};
use sat_astro::radiation::registry::{install_field, install_igrf, install_map};
use sat_astro::radiation::{BeltField, TrappedMap, TrappedModel};

/// この群のデータをまとめて取得・取り込むときの ID（`fune data fetch radiation`）。
pub(super) const GROUP_ID: &str = "radiation";

/// 群に含まれる行の ID（表の順）。
pub(super) const GROUP_IDS: [&str; 7] = [
    "ap8min",
    "ap8max",
    "ae8min",
    "ae8max",
    "jensen_cain_1960",
    "gsfc_1966",
    "igrf14",
];

const REPO: &str = "akira-nkjm/iwafune";
const TAG: &str = "data-radiation-v1";

/// NSSDC PT-11B の出所と参考文献（地図と古い磁場の係数に共通）。
macro_rules! nssdc {
    ($what:literal) => {
        concat!(
            $what,
            ". NSSDC data set PT-11B (D. Bilitza, NASA GSFC/NSSDC, March 1996), from the NASA CCMC ModelWeb archive; NASA states no licence for these files; redistributed unmodified from NASA's public archive. ",
            "References: Sawyer & Vette 1976 (NSSDC/WDC-A-R&S 76-06), Vette 1991 (NSSDC/WDC-A-R&S 91-24), Heynderickx & Beliaev 1995 (J. Spacecraft and Rockets 32, 190-192)"
        )
    };
}

const USED_BY: &str = "Trapped proton and electron fluxes (orbit.trapped_flux)";

fn install_map_bytes(model: TrappedModel, bytes: &[u8]) -> std::result::Result<(), String> {
    let map = TrappedMap::from_ascii_bytes(bytes).map_err(|e| e.to_string())?;
    install_map(model, map).map_err(|e| e.to_string())
}

fn install_ap8min(bytes: Vec<u8>) -> std::result::Result<(), String> {
    install_map_bytes(TrappedModel::Ap8Min, &bytes)
}
fn install_ap8max(bytes: Vec<u8>) -> std::result::Result<(), String> {
    install_map_bytes(TrappedModel::Ap8Max, &bytes)
}
fn install_ae8min(bytes: Vec<u8>) -> std::result::Result<(), String> {
    install_map_bytes(TrappedModel::Ae8Min, &bytes)
}
fn install_ae8max(bytes: Vec<u8>) -> std::result::Result<(), String> {
    install_map_bytes(TrappedModel::Ae8Max, &bytes)
}

fn install_jensen_cain(bytes: Vec<u8>) -> std::result::Result<(), String> {
    let model = GaussModel::from_jensen_cain_bytes(&bytes).map_err(|e| e.to_string())?;
    install_field(BeltField::JensenCain1960, model).map_err(|e| e.to_string())
}

/// GSFC 12/66 は 1970 年に外挿した固定の場にして登録する（AP-8 の太陽活動極大の地図が前提にする場）。
fn install_gsfc(bytes: Vec<u8>) -> std::result::Result<(), String> {
    let model = GaussModel::from_gsfc1266_bytes(&bytes, 1970.0).map_err(|e| e.to_string())?;
    install_field(BeltField::Gsfc1266To1970, model).map_err(|e| e.to_string())
}

fn install_igrf_bytes(bytes: Vec<u8>) -> std::result::Result<(), String> {
    let table = IgrfTable::from_bytes(&bytes).map_err(|e| e.to_string())?;
    install_igrf(table).map_err(|e| e.to_string())
}

pub(super) static AP8_MIN: Dataset = Dataset::new(ReleaseSpec {
    id: "ap8min",
    file: "ap8min.asc",
    repo: REPO,
    tag: TAG,
    sha256: "e381b3aaa88a120e6ee9e8cc0645d0205ca947924d2bf32e86a105044749ebf8",
    size: 102_318,
    used_by: USED_BY,
    license: nssdc!("AP-8 MIN trapped proton map (solar minimum, epoch 1964)"),
    installer: install_ap8min,
});

pub(super) static AP8_MAX: Dataset = Dataset::new(ReleaseSpec {
    id: "ap8max",
    file: "ap8max.asc",
    repo: REPO,
    tag: TAG,
    sha256: "e21fced8e9c2e97d6bf4824173bed1af803b9f52ca261c49106b0c58e609a7de",
    size: 100_542,
    used_by: USED_BY,
    license: nssdc!("AP-8 MAX trapped proton map (solar maximum, epoch 1970)"),
    installer: install_ap8max,
});

pub(super) static AE8_MIN: Dataset = Dataset::new(ReleaseSpec {
    id: "ae8min",
    file: "ae8min.asc",
    repo: REPO,
    tag: TAG,
    sha256: "4cea68a560a98e3efab4ec3b02a78a026c077453f179606a9a598698cfa34734",
    size: 81_254,
    used_by: USED_BY,
    license: nssdc!("AE-8 MIN trapped electron map (solar minimum, epoch 1964)"),
    installer: install_ae8min,
});

pub(super) static AE8_MAX: Dataset = Dataset::new(ReleaseSpec {
    id: "ae8max",
    file: "ae8max.asc",
    repo: REPO,
    tag: TAG,
    sha256: "73cf3ad873c8adea48c27eb7628621a03937d9f6c17b55805d32692e636815e8",
    size: 83_596,
    used_by: USED_BY,
    license: nssdc!("AE-8 MAX trapped electron map (solar maximum, epoch 1970)"),
    installer: install_ae8max,
});

pub(super) static JENSEN_CAIN: Dataset = Dataset::new(ReleaseSpec {
    id: "jensen_cain_1960",
    file: "jensen_cain_62.dat",
    repo: REPO,
    tag: TAG,
    sha256: "5d50984f71a9372d1d8dd366014ec3014ce02d9dc87169ded9d5954af80a01f3",
    size: 2_268,
    used_by: "Geomagnetic field (Jensen & Cain 1960) the AP-8 MIN and AE-8 maps are defined in (orbit.trapped_flux)",
    license: nssdc!("Jensen & Cain 1960 geomagnetic field coefficients"),
    installer: install_jensen_cain,
});

pub(super) static GSFC_1966: Dataset = Dataset::new(ReleaseSpec {
    id: "gsfc_1966",
    file: "gsfc66.dat",
    repo: REPO,
    tag: TAG,
    sha256: "413cad61300fbe4e44e586ad52d1c29f44a9ce4732b9192e378df026e102985c",
    size: 36_612,
    used_by: "Geomagnetic field (GSFC 12/66, extrapolated to 1970) the AP-8 MAX map is defined in (orbit.trapped_flux)",
    license: nssdc!("GSFC 12/66 geomagnetic field coefficients"),
    installer: install_gsfc,
});

pub(super) static IGRF14: Dataset = Dataset::new(ReleaseSpec {
    id: "igrf14",
    file: "igrf14coeffs.txt",
    repo: REPO,
    tag: TAG,
    sha256: "8f8d88403028fc4ee92c4f38d97b46e0a87e2cfc496045b43c9e26c1d6b0903c",
    size: 42_411,
    used_by: "Present-day geomagnetic field (IGRF-14, 1900 to 2030)",
    license: "International Association of Geomagnetism and Aeronomy (2024), IGRF-14, Zenodo, doi:10.5281/zenodo.14012302, CC BY 4.0; Beggan et al. 2026, doi:10.1186/s40623-025-02360-0",
    installer: install_igrf_bytes,
});
