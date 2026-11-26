//! 板の温度の分布（`thermal::plate_view`）の非定常の場: 升ごとの最低・最高・走行の終わりの温度。

use super::{ThermalTransientInput, ThermalTransientNodeResult, ThermalTransientOutput};
use crate::spec::{AnalysisError, ChannelKind};
use crate::thermal::plate_view::{
    PlateFieldSource, ThermalPlateQuantity, plate_temperatures, to_detail,
};

/// 板の温度の分布（升ごとの最低・最高・走行の終わりの温度の 3 つの場。全履歴は `series`）。板に分けていなければ詳細は無い。
pub(super) fn plate_detail(
    input: &ThermalTransientInput,
    output: &ThermalTransientOutput,
) -> Result<Option<serde_json::Value>, AnalysisError> {
    let of = |quantity, f: &dyn Fn(&ThermalTransientNodeResult) -> f64| PlateFieldSource {
        quantity,
        case: None,
        temperatures: output
            .nodes
            .iter()
            .map(|n| (n.node.as_str(), f(n)))
            .collect(),
    };
    // 終わりの温度は節点の温度の系列の最後のサンプル（系列の key は節点の ID）
    let end = PlateFieldSource {
        quantity: ThermalPlateQuantity::End,
        case: None,
        temperatures: output
            .series
            .channels
            .iter()
            .filter(|c| c.kind == ChannelKind::NodeTemperature)
            .filter_map(|c| Some((c.key.as_str(), *c.values.last()?)))
            .collect(),
    };
    let sources = vec![
        of(ThermalPlateQuantity::Min, &|n| n.min.value()),
        of(ThermalPlateQuantity::Max, &|n| n.max.value()),
        end,
    ];
    to_detail(plate_temperatures(&input.network.plates, sources)?)
}
