//! 熱の結合・表面材・面の割合の下書き → `SetSetting` の列（`CommandText::ThermalSet`）。パスの組み立てと値の型は
//! `fune set|unset analysis.thermal.…` と同じ（`SetText` を通す）ので、同じ編集は CLI でも HTTP でも同じファイルになる。

use super::text::{invalid, setting_text_value, thermal_field_type};
use crate::dto::{ThermalCellText, ThermalCouplingKind, ThermalSetText};
use crate::error::Result;
use fune_model::design::setting_dims::{THERMAL_CONDUCTIVE, THERMAL_RADIATIVE};
use fune_model::path::SettingPath;
use fune_ops::Op;

/// 下書きのセル → `SetSetting` の列（セルの並びのまま。値の無いセルは消す）。適用はしない。
pub fn thermal_set_ops(t: &ThermalSetText) -> Result<Vec<Op>> {
    let mut ops = Vec::new();
    for cell in &t.cells {
        cell_ops(cell, &mut ops)?;
    }
    if ops.is_empty() {
        return Err(invalid("No thermal edit to apply"));
    }
    Ok(ops)
}

fn thermal_path(keys: &[&str]) -> SettingPath {
    let mut segments = vec!["thermal".to_owned()];
    segments.extend(keys.iter().map(|k| (*k).to_owned()));
    SettingPath {
        scope: fune_model::path::SettingScope::Analysis,
        segments,
    }
}

fn cell_ops(cell: &ThermalCellText, ops: &mut Vec<Op>) -> Result<()> {
    match cell {
        ThermalCellText::Coupling { kind, a, b, value } => {
            let pair = [a.trim(), b.trim()];
            let path = match kind {
                ThermalCouplingKind::Conductive => THERMAL_CONDUCTIVE.at(&pair),
                ThermalCouplingKind::Radiative => THERMAL_RADIATIVE.at(&pair),
            };
            ops.push(set(path, value.as_deref())?);
        }
        ThermalCellText::Coating {
            id,
            label,
            absorptivity,
            emissivity,
        } => {
            let id = id.trim();
            let fields = [
                ("label", label),
                ("absorptivity", absorptivity),
                ("emissivity", emissivity),
            ];
            for (field, text) in fields {
                if let Some(text) = text {
                    ops.push(set(thermal_path(&["coating", id, field]), Some(text))?);
                }
            }
        }
        ThermalCellText::CoatingRemove { id } => {
            ops.push(set(thermal_path(&["coating", id.trim()]), None)?);
        }
        ThermalCellText::Surface {
            part,
            coating,
            fraction,
        } => ops.push(set(
            thermal_path(&["surface", part.trim(), coating.trim()]),
            fraction.as_deref(),
        )?),
        ThermalCellText::SurfaceRemove { part } => {
            ops.push(set(thermal_path(&["surface", part.trim()]), None)?);
        }
    }
    Ok(())
}

/// 1 つのパスへの `SetSetting`（`text` が None なら消す）。型は `fune set` と同じ表（`thermal_field_type`）から決める。
fn set(path: SettingPath, text: Option<&str>) -> Result<Op> {
    let value = match text {
        Some(text) => Some(setting_text_value(text, thermal_field_type(&path))?),
        None => None,
    };
    Ok(Op::SetSetting {
        path: path.to_string(),
        value,
    })
}
