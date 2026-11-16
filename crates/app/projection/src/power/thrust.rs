//! `power.thrust_balance`: 電力収支の入力（`power.energy_balance` の射影）と、数値伝搬のマヌーバの噴射の仕方
//! （`[constraints]` の `mode`・`sunlit_only`・`duty`）。噴射のモードを書いた数値伝搬のマヌーバが無ければ不足の入力。

use super::energy_balance;
use crate::context::Context;
use crate::missing::MissingInputs;
use fune_model::Target;
use fune_model::design::ManeuverMethod;
use sat_analysis::power::thrust_balance::{ThrustBalanceInput, ThrustLoad};
use sat_units::typed::Fraction;

/// 電力収支の入力と、噴射のモードを書いた数値伝搬のマヌーバ（実行の順）。
pub fn thrust_balance(ctx: &Context<'_>) -> Result<ThrustBalanceInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let thrusts: Vec<ThrustLoad> = ctx
        .design
        .ordered_maneuvers()
        .into_iter()
        .filter(|x| x.method == ManeuverMethod::Propagated)
        .filter_map(|x| {
            Some(ThrustLoad {
                maneuver: x.id.to_string(),
                mode: x.constraints.mode.as_ref()?.to_string(),
                sunlit_only: x.constraints.sunlit_only,
                duty: x.constraints.duty.map(Fraction::new),
            })
        })
        .collect();
    if thrusts.is_empty() {
        m.push(
            Target::Maneuvers,
            "No maneuver with numerical propagation (method = propagated) that names a burn mode (constraints.mode)",
        );
    }
    let balance = energy_balance(ctx).map_err(|e| m.extend(e)).ok();
    m.finish(|| {
        Some(ThrustBalanceInput {
            balance: balance?,
            thrusts,
        })
    })
}
