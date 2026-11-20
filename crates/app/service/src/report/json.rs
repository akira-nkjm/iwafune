//! 解析の出力（JSON）の読み取り。欠けた欄は None・空の列として扱い、読み取りでは失敗させない。

use serde_json::Value;

/// 欄の値。
pub(super) fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key).filter(|x| !x.is_null())
}

/// 数値の欄。
pub(super) fn num(v: &Value, key: &str) -> Option<f64> {
    get(v, key)?.as_f64()
}

/// 文字列の欄。
pub(super) fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    get(v, key)?.as_str()
}

/// 配列の欄（無ければ空）。
pub(super) fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    get(v, key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// 数値の配列の欄（数値でない要素は NaN）。
pub(super) fn numbers(v: &Value, key: &str) -> Vec<f64> {
    list(v, key)
        .iter()
        .map(|x| x.as_f64().unwrap_or(f64::NAN))
        .collect()
}
