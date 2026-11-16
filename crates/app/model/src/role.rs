//! 部品の役割と慣例の停止状態の綴りの正本。

use crate::enum_text::enum_text;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

enum_text! {
    /// 解析が部品を見分ける役割。種別の継承で引き継ぐ。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
    pub enum Role: "part role" {
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
        PropulsionThruster = "propulsion_thruster",
        PropellantTank = "propellant_tank",
    }
}

/// 慣例の停止状態。状態の語彙そのものは種別ごとに自由に定義する。
pub const STATE_OFF: &str = "off";

/// 慣例の主な「オン」の状態。モード表でセルを押してオンにするときの状態に使う（語彙に無ければ停止以外の最初の状態。
/// 選び方は `fune_service::query::views::stop_and_main_states` だけ）。
pub const STATE_MAIN: &str = "on";

/// スラスタ（role = thruster）の噴射している状態。モードでこの状態にある時間の割合だけ噴射するとみなす
/// （推力誤差の擾乱）。推進系のスラスタ（role = propulsion_thruster）の噴射中の消費電力もこの状態の値。
/// 組み込みの種別 thruster・propulsion_thruster の語彙にあることは `builtin` のテストで確かめ、独自の種別で語彙に無いものは
/// 検査（`def.thruster_firing_state`）が warning にする。
pub const STATE_THRUSTER_FIRING: &str = "on";
