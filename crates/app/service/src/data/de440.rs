//! DE440 の暦表: 表（`data::RELEASE_DATASETS`）の最初の行と、sat-astro への登録。

use super::release::{Dataset, ReleaseSpec};

/// DE440 の行。
pub(super) static DE440: Dataset = Dataset::new(ReleaseSpec {
    id: "de440",
    file: "de440.bin",
    repo: "akira-nkjm/iwafune",
    tag: "data-de440-v1",
    sha256: "75efbddb8ca6b1965a624a12e1cf7b7a9a231982b51106a1ecf000bdc8069d50",
    size: 20_715_072,
    used_by: "Celestial body positions (sun, moon and planets) for 1849 to 2150; analytic expressions are used outside that range",
    license: "JPL DE440 planetary ephemeris, converted to the format sat-astro reads",
    installer: install_de440_bytes,
});

/// DE440 のバイト列を sat-astro に渡す。
fn install_de440_bytes(bytes: Vec<u8>) -> std::result::Result<(), String> {
    let table = sat_astro::ephem::De440::from_bytes(bytes).map_err(|e| e.to_string())?;
    sat_astro::ephem::install_de440(table).map_err(|e| e.to_string())
}
