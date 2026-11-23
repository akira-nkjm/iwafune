//! 打ち上げ条件の読み出し。`conditions/launch.toml` が選ぶ打ち上げ機・ディスペンサー（カタログ品）の属性を SI で引く。
//!
//! **値の優先順の規則はここだけ**: `conditions/launch.toml` に直接書いた値（荷重など。読むのは各射影）→ ディスペンサー →
//! 打ち上げ機（衛星に近い接点が先。`LaunchInterface::NEAREST_FIRST`）。要求は限度を厳しくする側にだけ働き
//! （質量・固有振動数。[`mass_limits`]・[`frequency_limits`]）、効く 1 つの選び方は解析 `structure.launch_fit` が持つ
//! （`tightest_max_mass`・`tightest_min_frequency`）。値の出所は文字列（`launcher local/lv_alpha`）で持ち、判定の文に出す。

use crate::context::Context;
use crate::missing::MissingInputs;
use fune_model::Target;
use fune_model::design::launch::attr;
use fune_model::design::{BudgetKind, CatalogItem, LaunchInterface};
use fune_model::path::{AttrPath, SettingPath};
use fune_model::resolve::{Cell, ResolvedValue, SiValue, resolve_catalog_item};
use sat_analysis::structure::launch_fit::{LaunchFitFrequencyLimit, LaunchFitMassLimit};
use sat_units::typed::{Frequency, Mass, Qty};
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
