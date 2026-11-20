//! 新しい地上局のファイル（`conditions/ground/<ID>.toml`）の正規形。局の値の書き換えは設定の書き込み（`entities::set_setting`）。

use super::doc::string;
use super::entities::quantity;
use crate::profile as format;
use fune_model::design::{GroundLinkSettings, GroundStationSettings};
use toml_edit::{DocumentMut, Item, Table};

/// 新しい地上局のファイル（正規形: 見出しのキー（label → 位置 → 最低仰角）→ `[link.<帯域>]`）。`id` はファイル名で決まるので書かない。
pub fn new_ground_text(station: &GroundStationSettings) -> String {
    let mut doc = DocumentMut::new();
    let top = doc.as_table_mut();
    if let Some(label) = &station.label {
        top.insert("label", string(label));
    }
    let quantities = [
        ("latitude", &station.latitude),
        ("longitude", &station.longitude),
        ("altitude", &station.altitude),
        ("min_elevation", &station.min_elevation),
    ];
    for (key, q) in quantities {
        if let Some(q) = q {
            top.insert(key, quantity(q));
        }
    }
    if !station.links.is_empty() {
        let mut links = Table::new();
        links.set_implicit(true);
        for (band, link) in &station.links {
            links.insert(band, Item::Table(link_table(link)));
        }
        top.insert("link", Item::Table(links));
    }
    format::finish(doc)
}

/// 帯域 1 つの表（`CommLink` と同じ並び: アンテナ利得 → 雑音温度 → G/T → 送信電力 → 損失）。
fn link_table(link: &GroundLinkSettings) -> Table {
    let mut t = Table::new();
    let rows = [
        ("antenna_gain", &link.antenna_gain),
        ("noise_temperature", &link.noise_temperature),
        ("g_over_t", &link.g_over_t),
        ("tx_power", &link.tx_power),
        ("line_loss", &link.line_loss),
    ];
    for (key, q) in rows {
        if let Some(q) = q {
            t.insert(key, quantity(q));
        }
    }
    t
}
