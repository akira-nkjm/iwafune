//! 直前に JSON へ書き出した [`crate::typed::Qty`] の次元名（スレッドごと）。
//!
//! `Qty` は JSON では SI の数値だけになるので、出力側（service の DTO など）が次元を添えたいときに使う。
//! [`crate::typed::Qty`] の `Serialize` が書き出す直前に次元を記録し、[`take_last_dimension`] が取り出して消す。
//! JSON の中身は変えない。

use crate::Dimension;
use core::cell::Cell;
use std::sync::Mutex;
use std::vec::Vec;

std::thread_local! {
    static LAST: Cell<Option<Recorded>> = const { Cell::new(None) };
}

/// 記録したもの: 次元か、表示の種類の名前（[`crate::typed::Fraction::KIND`]）。
#[derive(Clone, Copy)]
enum Recorded {
    Dimension(Dimension),
    Kind(&'static str),
}

/// `Serialize` から呼ぶ。
pub(crate) fn record(dimension: Dimension) {
    LAST.with(|c| c.set(Some(Recorded::Dimension(dimension))));
}

/// `Serialize` から呼ぶ（次元の代わりに表示の種類を持つ量。[`crate::typed::Fraction`]）。
pub(crate) fn record_kind(kind: &'static str) {
    LAST.with(|c| c.set(Some(Recorded::Kind(kind))));
}

/// 直前に書き出した `Qty` の次元名を取り出す（取り出すと消える）。
///
/// 名前は [`crate::format::dimension_name`]（Energy と Torque は同じ次元なので Energy）。
/// 表示の種類を持つ量（[`crate::typed::Fraction`]）は種類の名前（`Fraction`）を返す。
/// 名前の無い次元は SI の組立単位の式（[`crate::format::si_unit_expression`]）を返す。
/// 式の文字列は次元ごとに 1 度だけ確保して使い回す（種類は有限なので増え続けない）。
pub fn take_last_dimension() -> Option<&'static str> {
    LAST.with(Cell::take).map(|r| match r {
        Recorded::Dimension(d) => name_of(d),
        Recorded::Kind(kind) => kind,
    })
}

fn name_of(dimension: Dimension) -> &'static str {
    crate::format::dimension_name(dimension).unwrap_or_else(|| intern(dimension))
}

fn intern(dimension: Dimension) -> &'static str {
    static CACHE: Mutex<Vec<(Dimension, &'static str)>> = Mutex::new(Vec::new());
    let mut cache = match CACHE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some((_, s)) = cache.iter().find(|(d, _)| *d == dimension) {
        return s;
    }
    let leaked: &'static str =
        std::boxed::Box::leak(crate::format::si_unit_expression(dimension).into_boxed_str());
    cache.push((dimension, leaked));
    leaked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed::{Fraction, Length, Power, Qty, Torque};

    fn json<T: serde::Serialize>(v: &T) -> std::string::String {
        serde_json::to_string(v).expect("JSON")
    }

    #[test]
    fn serializing_a_qty_records_its_dimension() {
        assert_eq!(take_last_dimension(), None);
        let p: Qty<Power> = Qty::from_raw(12.5);
        assert_eq!(json(&p), "12.5");
        assert_eq!(take_last_dimension(), Some("Power"));
        assert_eq!(take_last_dimension(), None);
        let l: Qty<Length> = Qty::from_raw(1.0);
        let _ = json(&l);
        assert_eq!(take_last_dimension(), Some("Length"));
        // Torque は Energy と同じ次元
        let t: Qty<Torque> = Qty::from_raw(0.01);
        let _ = json(&t);
        assert_eq!(take_last_dimension(), Some("Energy"));
        // 比は表示の種類の名前
        assert_eq!(json(&Fraction::new(0.25)), "0.25");
        assert_eq!(take_last_dimension(), Some(Fraction::KIND));
    }

    #[test]
    fn unnamed_dimension_falls_back_to_si_expression() {
        let d = <Power as crate::typed::Dimension>::DIM
            .try_mul(<Length as crate::typed::Dimension>::DIM)
            .expect("dimension");
        record(d);
        let name = take_last_dimension().expect("recorded");
        assert_eq!(name, "m^3*kg/s^3");
        record(d);
        assert!(core::ptr::eq(
            take_last_dimension().expect("second time"),
            name
        ));
    }
}
