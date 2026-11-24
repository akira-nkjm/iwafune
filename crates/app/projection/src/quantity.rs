//! 書かれた量（`{ value, unit }`）→ 型付きの SI 量。次元は型から決め、合わなければ不足入力として記録する。
//! 単位の解析と換算は `fune_model::value::units`（中身は sat-units）に任せる。

use crate::missing::MissingInputs;
use fune_model::Target;
use fune_model::design::setting_dims::SettingQuantity;
use fune_model::value::Quantity;
use sat_units::typed::{Decibel, Dimension, Qty};

/// 書かれた量を `Qty<D>` にする。無いとき・次元が違うときは `m` に記録して `None`。
pub fn qty<D: Dimension>(
    q: Option<&Quantity>,
    target: impl Into<Target>,
    reason: &str,
    m: &mut MissingInputs,
) -> Option<Qty<D>> {
    let Some(q) = q else {
        m.push(target, reason);
        return None;
    };
    match si::<D>(q) {
        Ok(v) => Some(v),
        Err(e) => {
            m.push(target, format!("{reason} ({e})"));
            None
        }
    }
}

/// 書かれた量を `Qty<D>` にする（任意の入力。無ければ `None`、次元が違えば記録）。
pub fn qty_opt<D: Dimension>(
    q: Option<&Quantity>,
    target: impl Into<Target>,
    m: &mut MissingInputs,
) -> Option<Qty<D>> {
    let q = q?;
    match si::<D>(q) {
        Ok(v) => Some(v),
        Err(e) => {
            m.push(target, e);
            None
        }
    }
}

/// 設定の量を `Qty<D>` にする。次元の型 `D` は設定の次元の表の欄 `key`（`fune_model::design::setting_dims`。
/// 検査と同じ表）から決まる。`fill` はパスの形の `*` を前から埋める区切り（地上局の ID など）。
pub fn setting<D: Dimension>(
    key: &SettingQuantity<D>,
    fill: &[&str],
    q: Option<&Quantity>,
    reason: &str,
    m: &mut MissingInputs,
) -> Option<Qty<D>> {
    qty(q, key.at(fill), reason, m)
}

/// 任意の設定の量を `Qty<D>` にする（無ければ `None`、次元が違えば記録）。型は [`setting`] と同じく表の欄から。
pub fn setting_opt<D: Dimension>(
    key: &SettingQuantity<D>,
    fill: &[&str],
    q: Option<&Quantity>,
    m: &mut MissingInputs,
) -> Option<Qty<D>> {
    qty_opt(q, key.at(fill), m)
}

/// 書かれた単位付きのベクトル（`{ x, y, z, unit }`）を、次元 `D` の SI の `Vec3` にする。無いとき・単位の次元が違うときは
/// `m` に記録して `None`（成分ごとにでなく 1 回だけ記録する）。
pub fn vec_qty<D: Dimension>(
    v: Option<&fune_model::value::Vector3>,
    target: impl Into<Target>,
    reason: &str,
    m: &mut MissingInputs,
) -> Option<sat_astro::math::Vec3> {
    let Some(v) = v else {
        m.push(target, reason);
        return None;
    };
    let unit = v.unit.clone().unwrap_or_default();
    let comp = |x: f64| si::<D>(&Quantity::new(x, unit.clone())).map(|q| q.value());
    match (comp(v.x), comp(v.y), comp(v.z)) {
        (Ok(x), Ok(y), Ok(z)) => Some(sat_astro::math::Vec3::new(x, y, z)),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
            m.push(target, format!("{reason} ({e})"));
            None
        }
    }
}

/// 次元を検査して SI の `Qty<D>` にする（検査と換算は `Quantity::si_as`）。
pub fn si<D: Dimension>(q: &Quantity) -> Result<Qty<D>, String> {
    q.si_as(&sat_units::format::dimension_label(D::DIM))
        .map(Qty::from_raw)
        .map_err(|e| e.to_string())
}

/// 書かれた量を dB にする（単位どおりに SI の線形の比へ直してから `sat_units::ratio_to_db`。単位の無い値は線形の比、`dB` と書けば dB）。
/// 単位が読めない・比が正でないときは理由を返す。
fn read_db(q: &Quantity) -> Result<Decibel, String> {
    let ratio = fune_model::value::units::to_si(q.value, &q.unit).map_err(|e| e.to_string())?;
    ratio_db(ratio).ok_or_else(|| format!("{} is not a positive ratio", q.value))
}

/// 任意の dB の量（損失など）。**書かれていなければ `None`（記録しない）。書かれていて読めなければ、対象を名指しして不足入力に記録して `None`**
/// （読めないものを黙って無いことにすると、損失が無いものとして余裕が楽観になる）。
pub fn db_opt(
    q: Option<&Quantity>,
    target: impl Into<Target>,
    m: &mut MissingInputs,
) -> Option<Decibel> {
    let q = q?;
    match read_db(q) {
        Ok(v) => Some(v),
        Err(e) => {
            m.push(target, format!("Cannot read the value as dB ({e})"));
            None
        }
    }
}

/// 必須の dB の量。無いときも読めないときも、対象を名指しして不足入力に記録して `None`。
pub fn db_need(
    q: Option<&Quantity>,
    target: impl Into<Target>,
    reason: &str,
    m: &mut MissingInputs,
) -> Option<Decibel> {
    let Some(q) = q else {
        m.push(target, reason);
        return None;
    };
    match read_db(q) {
        Ok(v) => Some(v),
        Err(e) => {
            m.push(target, format!("{reason} ({e})"));
            None
        }
    }
}

/// 解決済みの線形の比を dB にする。比が正でなければ、対象を名指しして不足入力に記録して `None`。
pub fn ratio_db_need(
    ratio: f64,
    target: impl Into<Target>,
    reason: &str,
    m: &mut MissingInputs,
) -> Option<Decibel> {
    let db = ratio_db(ratio);
    if db.is_none() {
        m.push(
            target,
            format!("{reason} ({ratio} is not a positive ratio)"),
        );
    }
    db
}

/// 線形の比（解決済みの値）を dB に（`sat_units::ratio_to_db`）。正でなければ `None`。
pub fn ratio_db(ratio: f64) -> Option<Decibel> {
    (ratio > 0.0).then(|| Decibel::new(sat_units::ratio_to_db(ratio)))
}

/// SI の数値から `Qty<D>`（resolve 済みの SI 値を型に載せる）。
pub fn from_si<D: Dimension>(v: f64) -> Qty<D> {
    Qty::from_raw(v)
}

/// 単位付きのベクトル（`{ x, y, z, unit }`）→ SI の `Vec3`。単位が無ければそのまま。
pub fn vec_si(
    v: &fune_model::value::Vector3,
    target: impl Into<Target>,
    m: &mut MissingInputs,
) -> Option<sat_astro::math::Vec3> {
    let unit = v.unit.as_deref().unwrap_or("");
    let conv = |x: f64| fune_model::value::units::to_si(x, unit);
    match (conv(v.x), conv(v.y), conv(v.z)) {
        (Ok(x), Ok(y), Ok(z)) => Some(sat_astro::math::Vec3::new(x, y, z)),
        _ => {
            m.push(target, format!("Cannot parse unit `{unit}`"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use fune_model::design::BudgetKind;
    use fune_model::value::units::parse_dimension_name;
    use sat_units::typed::{Dimension, Information, Mass, Power};

    /// 配分（`budget.<種類>.*`）の次元は種類から決まる（`BudgetKind::dimension`。設定の次元の表には無い）。
    /// 射影とバジェットが型で読む次元と同じ。
    #[test]
    fn budget_kind_dimensions_are_the_projected_types() {
        let dim = |k: BudgetKind| parse_dimension_name(k.dimension()).ok();
        assert_eq!(dim(BudgetKind::Mass), Some(Mass::DIM));
        assert_eq!(dim(BudgetKind::Power), Some(Power::DIM));
        assert_eq!(dim(BudgetKind::Data), Some(Information::DIM));
    }
}
