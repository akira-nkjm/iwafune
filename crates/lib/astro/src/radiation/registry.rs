//! 読み込んだマップ・場のプロセス全体の登録（DE440 の `ephem::install_de440` と同じ考え方）。**登録は 1 つにつき 1 度だけ**で、
//! 利用者のキャッシュにある配布ファイルを読んだ側（`fune_service::data`）が起動時に登録する。解析は [`RadiationTables`] を通して引く。

use super::map::TrappedMap;
use super::model::{BeltField, TrappedModel};
use super::shielding::ShieldingData;
use crate::AstroError;
use crate::geomag::{GaussModel, IgrfTable};
use std::sync::OnceLock;

static MAPS: [OnceLock<TrappedMap>; 4] = [const { OnceLock::new() }; 4];
static FIELDS: [OnceLock<GaussModel>; 2] = [const { OnceLock::new() }; 2];
static IGRF: OnceLock<IgrfTable> = OnceLock::new();
static SHIELDING: OnceLock<ShieldingData> = OnceLock::new();

fn map_slot(model: TrappedModel) -> &'static OnceLock<TrappedMap> {
    &MAPS[TrappedModel::ALL
        .iter()
        .position(|&m| m == model)
        .unwrap_or(0)]
}

fn field_slot(field: BeltField) -> &'static OnceLock<GaussModel> {
    &FIELDS[BeltField::ALL.iter().position(|&f| f == field).unwrap_or(0)]
}

fn already(what: &str) -> AstroError {
    AstroError::InvalidInput(format!("{what} is already registered in this process"))
}

/// マップを登録する（1 度だけ）。
pub fn install_map(model: TrappedModel, map: TrappedMap) -> Result<(), AstroError> {
    map_slot(model).set(map).map_err(|_| already(model.id()))
}

/// マップが前提にする場を登録する（1 度だけ）。
pub fn install_field(field: BeltField, model: GaussModel) -> Result<(), AstroError> {
    field_slot(field)
        .set(model)
        .map_err(|_| already(&format!("{field:?}")))
}

/// IGRF の表を登録する（1 度だけ。現在の場が要る解析のため。AP-8・AE-8 の引きは使わない）。
pub fn install_igrf(table: IgrfTable) -> Result<(), AstroError> {
    IGRF.set(table).map_err(|_| already("IGRF"))
}

/// SHIELDOSE-2 の線量データベースを登録する（1 度だけ）。
pub fn install_shielding(data: ShieldingData) -> Result<(), AstroError> {
    SHIELDING.set(data).map_err(|_| already("SHIELDOSE-2 data"))
}

/// 登録した SHIELDOSE-2 の線量データベース。
pub fn shielding() -> Option<&'static ShieldingData> {
    SHIELDING.get()
}

/// 登録した IGRF の表。
pub fn igrf() -> Option<&'static IgrfTable> {
    IGRF.get()
}

/// 解析が引くマップと場の組。登録されたもの（[`RadiationTables::installed`]）か、試験が作ったものを借りる。
#[derive(Debug, Clone, Copy, Default)]
pub struct RadiationTables<'a> {
    /// [`TrappedModel::ALL`] の順のマップ
    pub maps: [Option<&'a TrappedMap>; 4],
    /// [`BeltField::ALL`] の順の場
    pub fields: [Option<&'a GaussModel>; 2],
}

impl<'a> RadiationTables<'a> {
    /// 登録されたもの。
    pub fn installed() -> RadiationTables<'static> {
        RadiationTables {
            maps: TrappedModel::ALL.map(|m| map_slot(m).get()),
            fields: BeltField::ALL.map(|f| field_slot(f).get()),
        }
    }

    /// マップ `model`。
    pub fn map(&self, model: TrappedModel) -> Option<&'a TrappedMap> {
        self.maps[TrappedModel::ALL.iter().position(|&m| m == model)?]
    }

    /// 場 `field`。
    pub fn field(&self, field: BeltField) -> Option<&'a GaussModel> {
        self.fields[BeltField::ALL.iter().position(|&f| f == field)?]
    }

    /// 足りないマップ・場の名前（4 つのマップと、組にする 2 つの場がそろっていれば空）。
    pub fn missing(&self) -> Vec<String> {
        let maps = TrappedModel::ALL
            .into_iter()
            .filter(|&m| self.map(m).is_none())
            .map(|m| m.id().to_owned());
        let fields = BeltField::ALL
            .into_iter()
            .filter(|&f| self.field(f).is_none())
            .map(|f| format!("{f:?}"));
        maps.chain(fields).collect()
    }
}
