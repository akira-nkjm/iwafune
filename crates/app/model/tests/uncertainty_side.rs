//! 値の不確かさのどちら側を読むか（`SiScalar::read`、#162）。

use fune_model::design::UncertaintySide::{Max, Min, Nominal};
use fune_model::resolve::SiScalar;

fn scalar(min: Option<f64>, max: Option<f64>) -> SiScalar {
    SiScalar {
        value: 10.0,
        min,
        max,
    }
}

/// `max` があれば max 側のケースはそれを読み、無ければ nominal。
#[test]
fn max_side_reads_max_or_falls_back_to_nominal() {
    assert_eq!(scalar(Some(8.0), Some(12.0)).read(Some(Max)), 12.0);
    assert_eq!(scalar(Some(8.0), None).read(Some(Max)), 10.0);
}

/// `min` があれば min 側のケースはそれを読み、無ければ nominal。
#[test]
fn min_side_reads_min_or_falls_back_to_nominal() {
    assert_eq!(scalar(Some(8.0), Some(12.0)).read(Some(Min)), 8.0);
    assert_eq!(scalar(None, Some(12.0)).read(Some(Min)), 10.0);
}

/// nominal・side の指定なし・ケースなしは、min / max があっても nominal。
#[test]
fn nominal_or_no_case_reads_nominal() {
    let s = scalar(Some(8.0), Some(12.0));
    assert_eq!(s.read(Some(Nominal)), 10.0);
    assert_eq!(s.read(None), 10.0);
}
