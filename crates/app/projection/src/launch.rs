//! 打ち上げ条件の読み出し。`conditions/launch.toml` が選ぶ打ち上げ機・ディスペンサー（カタログ品）の属性を SI で引く。
//!
//! **値の優先順の規則はここだけ**: `conditions/launch.toml` に直接書いた値（荷重など。読むのは各射影。列挙は [`LaunchValues::text_set`]。組の属性は同じ出所から）→ ディスペンサー →
//! 打ち上げ機（衛星に近い接点が先。`LaunchInterface::NEAREST_FIRST`）。要求は限度を厳しくする側にだけ働き
//! （質量・固有振動数。[`mass_limits`]・[`frequency_limits`]）、効く 1 つの選び方は解析 `structure.launch_fit` が持つ
//! （`tightest_max_mass`・`tightest_min_frequency`）。値の出所は文字列（`launcher local/lv_alpha`）で持ち、判定の文に出す。
//!
//! 質量の上限の候補は、スカラーの `max_payload_mass`（ディスペンサー → 打ち上げ機の順で最初のもの）、能力の曲線 `payload_mass_by_c3`
//! （同じ順で最初のもの。出発を打ち上げ機が受け持つ遷移があるときだけ、その C3 で引く）、質量バジェットの要求で、**全部が候補**として並び、
//! 厳しいほうが効く（スカラーは曲線の外の構造などの別の限度でもありうる）。

use crate::context::Context;
use crate::missing::MissingInputs;
use fune_model::Target;
use fune_model::design::launch::attr;
use fune_model::design::{BudgetKind, CatalogItem, LaunchInterface};
use fune_model::path::{AttrPath, SettingPath};
use fune_model::resolve::{Cell, ResolvedValue, SiValue, resolve_catalog_item};
use sat_analysis::structure::launch_fit::{
    CapabilityPoint, LaunchFitCapability, LaunchFitFrequencyLimit, LaunchFitMassLimit,
};
use sat_units::typed::{Frequency, Mass, Qty, SpecificEnergy};
use std::collections::BTreeMap;

/// 選ばれた品 1 つの SI の値。
struct Chosen<'a> {
    which: LaunchInterface,
    item: &'a CatalogItem,
    values: BTreeMap<String, ResolvedValue>,
}

/// 打ち上げ条件が選んだ打ち上げ機・ディスペンサー（衛星に近い接点が先）。
pub(crate) struct LaunchValues<'a> {
    ctx: &'a Context<'a>,
    chosen: Vec<Chosen<'a>>,
}

/// 引いた値と出所。
pub(crate) struct Found<T> {
    pub value: T,
    /// 出所（`launcher local/lv_alpha`）
    pub origin: String,
}

/// 同じ出所から読んだ文字列の属性の組（[`LaunchValues::text_set`]）。
pub(crate) struct TextSet {
    /// 出所（`launcher local/lv_alpha`・`conditions/launch.toml`）
    pub origin: String,
    /// 属性ごとの値（頼んだ並び。その出所に無ければ `None`）
    pub values: Vec<Option<String>>,
}

/// `conditions.launch.<欄>` のパス。
pub(crate) fn launch_path(keys: &[&str]) -> SettingPath {
    SettingPath::conditions(&["launch"]).child(keys)
}

impl<'a> LaunchValues<'a> {
    /// 選んだ品を読む（定義されていない参照は無いものとして扱う。検査が `ref.launcher`・`ref.dispenser` で知らせる）。
    pub fn new(ctx: &'a Context<'a>) -> Self {
        let chosen = LaunchInterface::NEAREST_FIRST
            .into_iter()
            .filter_map(|which| {
                let item = ctx.design.launch_item(which)?;
                ctx.note(launch_path(&[which.as_str()]));
                Some(Chosen {
                    which,
                    item,
                    values: resolve_catalog_item(ctx.design, item),
                })
            })
            .collect();
        Self { ctx, chosen }
    }

    /// 打ち上げ機かディスペンサーを 1 つでも選んでいるか。
    pub fn any(&self) -> bool {
        !self.chosen.is_empty()
    }

    /// 属性 `attr_id` の値をもつ最初の品の値（ディスペンサー → 打ち上げ機）。
    fn first(&self, attr_id: &str) -> Option<Found<&SiValue>> {
        self.chosen.iter().find_map(|c| {
            let v = c.values.get(attr_id)?;
            let Cell::Set(value) = &v.value else {
                return None;
            };
            self.ctx.note(Target::Catalog {
                id: c.item.id.clone(),
                attr: Some(AttrPath {
                    attr: attr_id.to_owned(),
                    state: None,
                }),
            });
            Some(Found {
                value,
                origin: format!("{} {}", c.which, c.item.id),
            })
        })
    }

    /// スカラーの属性（SI）。
    pub fn scalar(&self, attr_id: &str) -> Option<Found<f64>> {
        let f = self.first(attr_id)?;
        match f.value {
            SiValue::Scalar(s) => Some(Found {
                value: s.value,
                origin: f.origin,
            }),
            _ => None,
        }
    }

    /// 曲線の属性（SI。引数と値の対の並び）。
    pub fn curve(&self, attr_id: &str) -> Option<Found<Vec<(f64, f64)>>> {
        let f = self.first(attr_id)?;
        match f.value {
            SiValue::Curve { at, values } => Some(Found {
                value: at.iter().copied().zip(values.iter().copied()).collect(),
                origin: f.origin,
            }),
            _ => None,
        }
    }

    /// 互いに組になる文字列（列挙）の属性を、**同じ出所から**読む。先頭の属性 `attrs[0]` を持つ最初の出所
    /// （`conditions/launch.toml` に直接書いた値 `written` → ディスペンサー → 打ち上げ機）を選び、残りの属性はその出所だけから引く
    /// （別の出所の値は混ぜない）。返す値は `attrs` と同じ並びで、その出所が持たない属性は `None`。
    pub fn text_set(&self, attrs: &[(&str, Option<&str>)]) -> Option<TextSet> {
        let (lead, written) = *attrs.first()?;
        if written.is_some() {
            let values = attrs
                .iter()
                .map(|(attr_id, w)| {
                    w.inspect(|_| self.ctx.note(launch_path(&[attr_id])))
                        .map(str::to_owned)
                })
                .collect();
            return Some(TextSet {
                origin: "conditions/launch.toml".to_owned(),
                values,
            });
        }
        let c = self
            .chosen
            .iter()
            .find(|c| matches!(c.values.get(lead).map(|v| &v.value), Some(Cell::Set(_))))?;
        let values = attrs
            .iter()
            .map(|(attr_id, _)| {
                let v = c.values.get(*attr_id)?;
                let Cell::Set(SiValue::Enum(text)) = &v.value else {
                    return None;
                };
                self.ctx.note(Target::Catalog {
                    id: c.item.id.clone(),
                    attr: Some(AttrPath {
                        attr: (*attr_id).to_owned(),
                        state: None,
                    }),
                });
                Some(text.clone())
            })
            .collect();
        Some(TextSet {
            origin: format!("{} {}", c.which, c.item.id),
            values,
        })
    }

    /// ベクトルの属性（SI）。
    pub fn vec3(&self, attr_id: &str) -> Option<Found<[f64; 3]>> {
        let f = self.first(attr_id)?;
        match f.value {
            SiValue::Vec3(v) => Some(Found {
                value: *v,
                origin: f.origin,
            }),
            _ => None,
        }
    }
}

/// 質量の上限の候補（基本の限度 = 打ち上げ機・ディスペンサーの値、続いて質量のバジェットの上限の要求）。
pub(crate) fn mass_limits(ctx: &Context<'_>, launch: &LaunchValues<'_>) -> Vec<LaunchFitMassLimit> {
    let base = launch
        .scalar(attr::MAX_PAYLOAD_MASS)
        .map(|f| (f.origin, f.value));
    let requirement = ctx
        .budget_limit(BudgetKind::Mass)
        .map(|c| (format!("requirement {}", c.requirement), c.si));
    [base, requirement]
        .into_iter()
        .flatten()
        .map(|(origin, kg)| LaunchFitMassLimit {
            origin,
            value: Qty::<Mass>::from_raw(kg),
        })
        .collect()
}

/// 打ち上げ機・ディスペンサーの能力の曲線（`payload_mass_by_c3`）に、出発を打ち上げ機が受け持つ遷移の C3 を添える
/// （曲線が無ければ `None`。**質量の上限の候補になる曲線の読み方はここだけ**で、C3 で引くのと候補から選ぶのは解析）。
pub(crate) fn capability(
    launch: &LaunchValues<'_>,
    departure: &LauncherDeparture,
) -> Option<LaunchFitCapability> {
    let found = launch.curve(attr::PAYLOAD_MASS_BY_C3)?;
    Some(LaunchFitCapability {
        origin: found.origin,
        maneuver: departure.maneuver.clone(),
        c3: departure.c3,
        curve: found
            .value
            .into_iter()
            .map(|(c3, kg)| CapabilityPoint {
                c3: Qty::<SpecificEnergy>::from_raw(c3),
                mass: Qty::<Mass>::from_raw(kg),
            })
            .collect(),
    })
}

/// 出発を打ち上げ機が受け持つ遷移（マヌーバの ID と出発の C3）。
pub(crate) struct LauncherDeparture {
    pub maneuver: String,
    pub c3: Qty<SpecificEnergy>,
}

/// 最低固有振動数の候補（基本の限度 = 打ち上げ機・ディスペンサーの値、続いて `structure.modal_estimate` の判定
/// `lateral`・`axial` の要求）。`direction` は `lateral` か `axial`。
pub(crate) fn frequency_limits(
    ctx: &Context<'_>,
    launch: &LaunchValues<'_>,
    direction: &str,
) -> Vec<LaunchFitFrequencyLimit> {
    let attr_id = match direction {
        "axial" => attr::MIN_FREQUENCY_AXIAL,
        _ => attr::MIN_FREQUENCY_LATERAL,
    };
    let base = launch.scalar(attr_id).map(|f| (f.origin, f.value));
    let requirement = ctx
        .criterion("structure.modal_estimate", direction)
        .map(|c| (format!("requirement {}", c.requirement), c.si));
    [base, requirement]
        .into_iter()
        .flatten()
        .map(|(origin, hz)| LaunchFitFrequencyLimit {
            origin,
            value: Qty::<Frequency>::from_raw(hz),
        })
        .collect()
}

/// 打ち上げ機もディスペンサーも選んでいないとき、選べる場所を不足として記録する。
pub(crate) fn need_selection(launch: &LaunchValues<'_>, m: &mut MissingInputs) {
    if !launch.any() {
        m.push(
            launch_path(&["launcher"]),
            "Select a launcher (conditions.launch.launcher = \"<catalog>/<id>\", an item of kind `launcher`) or a dispenser (conditions.launch.dispenser)",
        );
    }
}
