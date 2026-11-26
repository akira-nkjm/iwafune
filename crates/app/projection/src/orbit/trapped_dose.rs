//! `orbit.trapped_dose` の射影と、`orbit.total_dose` が環境の表を持たない区間に使う計算した環境。
//! 軌道の 1 日分のサンプルの取り方（`orbit.trapped_flux` と同じ。ケースの β があれば当てる）だけを組み立てる。厚さ・幾何・核相互作用は
//! 解析の既定に任せる（物理の計算も既定値も持たない）。放射線帯のマップと SHIELDOSE-2 の表（配布データ）が読み込まれていなければ不足入力にして、
//! そろえる手順を示す。`orbit.total_dose` は**別の解析の出力を入力に使う**ので、先例（`comm.data_throughput` がデータ量の解析を走らせる）と同じく、
//! ここで解析を走らせて表だけを取り出す。

use super::trapped_flux::INSTALL_HINT;
use crate::{Context, MissingInputs};
use fune_model::Target;
use sat_analysis::orbit::{DoseDepthPoint, TrappedDose, TrappedDoseInput};
use sat_analysis::spec::Analysis;
use sat_astro::radiation::RadiationTables;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// 計算した環境の覚え（入力の JSON → 表）。同じ軌道の区間が何度も出る（フェーズ・GUI の問い合わせの繰り返し）ので、
/// 1 回の計算（数秒）を使い回す。データの読み込みはプロセスで一度きりなので、入力が同じなら結果も同じ。
type Memo = Mutex<HashMap<String, Option<Vec<DoseDepthPoint>>>>;
static MEMO: OnceLock<Memo> = OnceLock::new();

/// 覚えておく数の上限（超えたら捨てて作り直す）。
const MEMO_LIMIT: usize = 64;

/// SHIELDOSE-2 の表の名前（足りないものの表示。`fune_service::data` の行の ID と一致する）。
const SHIELDING_ID: &str = "shieldose2";

/// そろっていない配布データの名前（マップ・場と SHIELDOSE-2 の表。そろっていれば空）。
pub fn dose_data_missing() -> Vec<String> {
    let mut missing = RadiationTables::installed().missing();
    if sat_astro::radiation::registry::shielding().is_none() {
        missing.push(SHIELDING_ID.to_owned());
    }
    missing
}

fn input_of(sampling: sat_astro::timeline::OrbitSampling) -> TrappedDoseInput {
    TrappedDoseInput {
        sampling,
        thicknesses: None,
        geometry: None,
        nuclear: None,
    }
}

/// `orbit.trapped_dose` の入力。
pub fn trapped_dose(ctx: &Context<'_>) -> Result<TrappedDoseInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let sampling = crate::timeline::day_sampling(ctx, &mut m);
    let missing = dose_data_missing();
    if !missing.is_empty() {
        m.push(
            Target::Analysis {
                id: TrappedDose::ID.to_owned(),
            },
            format!(
                "The trapped-radiation data is not installed (missing: {}). Install it with: {INSTALL_HINT}",
                missing.join(", ")
            ),
        );
    }
    m.finish(|| sampling.map(input_of))
}

/// 区間の軌道の捕捉粒子による環境の表（遮蔽の厚さ → 線量率。太陽活動の極小・極大の悪い方）。
/// データがそろっていない・軌道が地球の周りでない・計算できないときは `None`（呼び出し側が「環境の表なし」の区間にする）。
pub fn computed_environment(ctx: &Context<'_>) -> Option<Vec<DoseDepthPoint>> {
    if !dose_data_missing().is_empty() {
        return None;
    }
    let mut ignored = MissingInputs::new();
    let input = input_of(crate::timeline::day_sampling(ctx, &mut ignored)?);
    let key = serde_json::to_string(&input).ok()?;
    let memo = MEMO.get_or_init(Default::default);
    if let Some(hit) = memo.lock().ok()?.get(&key) {
        return hit.clone();
    }
    let table = TrappedDose::run(&input)
        .ok()
        .map(|out| out.worst_case_table())
        .filter(|t| !t.is_empty());
    let mut map = memo.lock().ok()?;
    if map.len() >= MEMO_LIMIT {
        map.clear();
    }
    map.insert(key, table.clone());
    table
}
