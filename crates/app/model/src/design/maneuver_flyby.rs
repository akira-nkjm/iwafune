//! 日付つきの惑星間遷移の途中の通過（スイングバイ。マヌーバの `[[transfer.flybys]]`）の設計データと、旗（`--flyby`）の文字列の書き方・読み方。

use super::conditions::CentralBodyKind;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 日付つきの遷移の途中の通過 1 回（`[[transfer.flybys]]`）。通過の天体は出発・到着の天体と同じ親を回る惑星で、
/// 日は出発と到着の間（順に増える）。日付の読み方・順序・暦の範囲・天体の組の検査は暦を持つ fune-projection の側（`maneuver_dates`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(rename = "ManeuverFlyby")]
pub struct ManeuverFlyby {
    /// 通過する天体
    pub body: CentralBodyKind,
    /// 通過の日（ISO 8601、UTC。`departure` と同じ形）
    pub date: String,
    /// 近点の噴射（入りと出の超過速度の大きさの差）をこのマヌーバのスラスタが行うか（書かなければ行う = ΔV・推薬に数える）。
    /// `false` は動力なしのスイングバイのつもりで、差があっても数えず警告だけを出す
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub powered: Option<bool>,
    // 量（インライン表）は単純な値のあとに書かれるので、最後に置く
    /// 近点の高度の下限（長さ。天体の赤道半径から。書かなければ天体の種類ごとの慣例の既定。木星型の惑星は慣例が無いので書く）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_altitude: Option<Quantity>,
}

impl ManeuverFlyby {
    /// 近点の噴射を数えるか（書かなければ数える）。
    pub fn is_powered(&self) -> bool {
        self.powered.unwrap_or(true)
    }

    /// 打つ形（`fune maneuver add|set --flyby`。繰り返す旗）: `天体@日付` に、書かれていれば `,min_altitude=300 km`・`,powered=false` を続ける
    /// （例 `venus@2029-02-01T00:00:00Z,min_altitude=400 km`）。**旗の書き方はここだけ**（読み戻しは [`ManeuverFlyby::from_text`]。
    /// 日付の読み方・天体の組の検査は読み手の側）。
    pub fn to_text(&self) -> String {
        let mut text = format!("{}@{}", self.body.as_str(), self.date);
        if let Some(q) = &self.min_altitude {
            text.push_str(&format!(
                ",min_altitude={}",
                crate::value::quantity_to_text(q)
            ));
        }
        if let Some(p) = self.powered {
            text.push_str(&format!(",powered={p}"));
        }
        text
    }

    /// 打った形（[`ManeuverFlyby::to_text`]）から読む。形が違う・天体が読めない・量が読めない・未知のキーは英語の 1 文のエラー。
    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut parts = text.split(',');
        let head = parts.next().unwrap_or_default();
        let Some((body, date)) = head.split_once('@') else {
            return Err(format!(
                "A flyby is written BODY@DATE (e.g. venus@2029-02-01T00:00:00Z), not `{}`",
                text.trim()
            ));
        };
        let body = body
            .trim()
            .parse()
            .map_err(|e| format!("Flyby body: {e}"))?;
        let mut flyby = Self {
            body,
            date: date.trim().to_owned(),
            min_altitude: None,
            powered: None,
        };
        for part in parts {
            let Some((key, value)) = part.split_once('=') else {
                return Err(format!(
                    "Flyby option `{}` is not KEY=VALUE (min_altitude=300 km or powered=false)",
                    part.trim()
                ));
            };
            match key.trim() {
                "min_altitude" => {
                    flyby.min_altitude = Some(
                        crate::value::quantity_from_text(value)
                            .map_err(|e| format!("Flyby min_altitude: {e}"))?,
                    );
                }
                "powered" => {
                    flyby.powered = Some(value.trim().parse::<bool>().map_err(|_| {
                        format!(
                            "Flyby powered must be true or false, not `{}`",
                            value.trim()
                        )
                    })?);
                }
                other => {
                    return Err(format!(
                        "Unknown flyby option `{other}` (min_altitude, powered)"
                    ));
                }
            }
        }
        Ok(flyby)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flyby_text_round_trips() {
        let f = ManeuverFlyby {
            body: CentralBodyKind::Venus,
            date: "2029-02-01T00:00:00Z".into(),
            min_altitude: Some(Quantity::new(400.0, "km")),
            powered: Some(false),
        };
        let text = f.to_text();
        assert_eq!(
            text,
            "venus@2029-02-01T00:00:00Z,min_altitude=400 km,powered=false"
        );
        assert_eq!(ManeuverFlyby::from_text(&text).unwrap(), f);
        let plain = ManeuverFlyby::from_text(" earth @ 2030-01-01T00:00:00Z ").unwrap();
        assert_eq!(plain.to_text(), "earth@2030-01-01T00:00:00Z");
        for bad in [
            "venus",
            "pluto@2029-02-01T00:00:00Z",
            "venus@2029-02-01T00:00:00Z,x=1",
            "venus@d,powered=maybe",
            "venus@d,min_altitude=big",
        ] {
            assert!(ManeuverFlyby::from_text(bad).is_err(), "{bad}");
        }
    }
}
