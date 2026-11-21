//! 新しい地上局のファイル（`conditions/ground/<ID>.toml`）の正規形。キーの並びは `GroundStationSettings` の serde の順
//! （共有の書き手 `records::record_doc`）が決める。局の値の書き換えは設定の書き込み（`entities::set_setting`）。

use super::records::{GROUND_SECTIONS, record_doc};
use crate::error::StoreError;
use crate::profile as format;
use fune_model::design::GroundStationSettings;

/// 新しい地上局のファイル（正規形: 見出しのキー（label → 位置 → 最低仰角）→ `[link.<帯域>]`）。`id` はファイル名で決まるので書かない。
pub fn new_ground_text(station: &GroundStationSettings) -> Result<String, StoreError> {
    let path = crate::layout::ground_file(&station.id);
    Ok(format::finish(record_doc(&path, station, GROUND_SECTIONS)?))
}
