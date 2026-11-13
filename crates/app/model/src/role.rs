//! 部品の役割と慣例の停止状態の綴りの正本。

use crate::enum_text::enum_text;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

enum_text! {
    /// 解析が部品を見分ける役割。種別の継承で引き継ぐ。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
    pub enum Role: "部品の役割" {
        Obc = "obc",
        Pcdu = "pcdu",
        Battery = "battery",
        SolarPanel = "solar_panel",
        ReactionWheel = "reaction_wheel",
        Magnetorquer = "magnetorquer",
        Thruster = "thruster",
        StarTracker = "star_tracker",
        SunSensor = "sun_sensor",
        Gyro = "gyro",
        Magnetometer = "magnetometer",
        Gps = "gps",
        Transceiver = "transceiver",
        Antenna = "antenna",
        Camera = "camera",
        Heater = "heater",
        Panel = "panel",
        Structure = "structure",
    }
}

/// 慣例の停止状態。状態の語彙そのものは種別ごとに自由に定義する。
pub const STATE_OFF: &str = "off";

/// スラスタ（role = thruster）の噴射している状態。モードでこの状態にある時間の割合だけ噴射するとみなす
/// （推力誤差の擾乱）。組み込みの種別 thruster の語彙にあることは `builtin` のテストで確かめる。
pub const STATE_THRUSTER_FIRING: &str = "on";
