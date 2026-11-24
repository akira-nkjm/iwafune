//! 解析ケースの欄 1 つ（`fune set cases.<ID>.<欄>`・`SetCaseField` の Op）。ケースを丸ごと書く `SetCase` と違い、書いた欄だけを変えて
//! ほかの欄は今のまま残す。**ケースの欄の名前（[`CaseFieldName`]）と、欄ごとの値の型（[`CaseField`]）はここだけ**
//! （service の打った文字列の読み方も、この列挙で match する）。名前の順はファイルのキーの正規の順（`Case` の宣言の順）。

use super::system::{Case, UncertaintySide};
use crate::enum_text::enum_text;
use crate::id::Id;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

enum_text! {
    /// 解析ケースの欄の名前（`cases/<ID>.toml` のキー。`fune set cases.<ID>.<欄>` のパスの最後の区切り）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum CaseFieldName: "case field" {
        Label = "label",
        Description = "description",
        Mode = "mode",
        Phase = "phase",
        Side = "side",
        ThermalCase = "thermal_case",
        Beta = "beta",
        Orbits = "orbits",
        SunDistance = "sun_distance",
        Epoch = "epoch",
    }
}

/// 解析ケースの型付きの欄（`SetCaseField` の Op が持つ）。`None` はその欄を外す。
/// フェーズ（`phase`）と時刻（`epoch`）は専用の Op（`SetCasePhase`・`SetCaseEpoch`）があるので、ここには無い
/// （service が欄の名前からそちらの Op にする）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum CaseField {
    Label(Option<String>),
    Description(Option<String>),
    Mode(Option<String>),
    Side(Option<UncertaintySide>),
    ThermalCase(Option<String>),
    Beta(Option<Quantity>),
    Orbits(Option<u32>),
    SunDistance(Option<Quantity>),
}

impl CaseField {
    /// 欄の名前。
    pub fn key(&self) -> CaseFieldName {
        match self {
            CaseField::Label(_) => CaseFieldName::Label,
            CaseField::Description(_) => CaseFieldName::Description,
            CaseField::Mode(_) => CaseFieldName::Mode,
            CaseField::Side(_) => CaseFieldName::Side,
            CaseField::ThermalCase(_) => CaseFieldName::ThermalCase,
            CaseField::Beta(_) => CaseFieldName::Beta,
            CaseField::Orbits(_) => CaseFieldName::Orbits,
            CaseField::SunDistance(_) => CaseFieldName::SunDistance,
        }
    }

    /// この欄だけを書き換えたケース（ほかの欄はそのまま）。
    pub fn applied(&self, case: &Case) -> Case {
        let mut out = case.clone();
        match self.clone() {
            CaseField::Label(v) => out.label = v,
            CaseField::Description(v) => out.description = v,
            CaseField::Mode(v) => out.mode = v.map(Id::new),
            CaseField::Side(v) => out.side = v,
            CaseField::ThermalCase(v) => out.thermal_case = v,
            CaseField::Beta(v) => out.beta = v,
            CaseField::Orbits(v) => out.orbits = v,
            CaseField::SunDistance(v) => out.sun_distance = v,
        }
        out
    }
}
