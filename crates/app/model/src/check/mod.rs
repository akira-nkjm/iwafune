//! 検証（severity 付きの診断）。**検証はここだけ**（store は構文の誤りだけを `load_diagnostics` に残す）。
//!
//! | 重さ | 例 |
//! |---|---|
//! | error | 単位の解析・次元の不一致、値の型（状態の軸）の不一致、存在しない ID の参照、取付の循環 |
//! | warning | 必須の属性の欠け（Unset）、許可単位の外、値域の外、モードの表に無い部品、配分の超過 |
//! | info | 参考（共有カタログを読んでいない など） |

mod budget;
mod defs;
mod parts;
mod settings;
mod settings_analysis;
mod system;
mod values;

pub use defs::attribute_def_problems;

/// 割合の合計を 1 とみなす許容差（デューティ比・モードの時間割合）。解析の側の `sat_analysis::modes::DUTY_TOLERANCE` と
/// 同じ値にする（fune-model は sat-analysis に依存できないので、fune-projection のテストが一致を確かめる）。
pub const SHARE_TOLERANCE: f64 = 1e-9;
pub use values::{Finding, check_quantity_dimension, check_value_against};

use crate::design::{Design, EntityKey};
use crate::diagnostic::{Diagnostic, Severity};

/// 設計全体を検証する（読み込み時の診断を含む）。
pub fn check(design: &Design) -> Vec<Diagnostic> {
    let mut ctx = Ctx {
        design,
        out: design.load_diagnostics.clone(),
    };
    defs::check_defs(&mut ctx);
    parts::check_parts(&mut ctx);
    parts::check_catalog(&mut ctx);
    system::check_modes(&mut ctx);
    system::check_nets(&mut ctx);
    system::check_harnesses(&mut ctx);
    system::check_cases_and_sources(&mut ctx);
    system::check_views(&mut ctx);
    budget::check_budget(&mut ctx);
    settings::check_mission(&mut ctx);
    settings_analysis::check_analysis(&mut ctx);
    settings::check_materials(&mut ctx);
    ctx.out
}

/// 診断を集める文脈。
pub(crate) struct Ctx<'a> {
    pub design: &'a Design,
    pub out: Vec<Diagnostic>,
}

impl Ctx<'_> {
    /// 診断を位置付きで足す。
    pub fn report(
        &mut self,
        severity: Severity,
        code: &str,
        entity: &EntityKey,
        keys: &[&str],
        path: Option<String>,
        message: impl Into<String>,
    ) {
        let loc = self.design.source_map.locate(entity, keys);
        let mut d = Diagnostic::new(severity, code, message).at(loc);
        d.path = path;
        self.out.push(d);
    }

    /// error を足す。
    pub fn error(
        &mut self,
        code: &str,
        entity: &EntityKey,
        keys: &[&str],
        path: Option<String>,
        message: impl Into<String>,
    ) {
        self.report(Severity::Error, code, entity, keys, path, message);
    }

    /// warning を足す。
    pub fn warning(
        &mut self,
        code: &str,
        entity: &EntityKey,
        keys: &[&str],
        path: Option<String>,
        message: impl Into<String>,
    ) {
        self.report(Severity::Warning, code, entity, keys, path, message);
    }
}
