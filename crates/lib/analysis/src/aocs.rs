//! AOCSの解析。各解析は 1 ファイル 1 つ（入出力の型と実装）。系の中で共有する計算はサブモジュールに置き、台帳に登録する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`disturbance`] | 擾乱トルク（重力傾斜・空力・太陽輻射圧・残留磁気・推力誤差） |
//! | [`wheels`] | ホイールの組（任意の台数・配置）への擬似逆行列での配分と容量 |
//! | [`thrusters`] | スラスタ 1 本の値と重心まわりのトルク（RCS・推力誤差で共有） |
//! | [`rw_unloading`] | `aocs.rw_unloading` RW アンロード頻度（RCS で捨てる 1 日の推薬と ΔV 換算も） |
//! | [`unload_time`] | `aocs.unload_time` MTQ・RCS でのアンロード時間 |
//! | [`sensor_fov`] | `aocs.sensor_fov` センサの禁止角への侵入 |
//! | [`slew_time`] | `aocs.slew_time` RW での姿勢変更にかかる時間 |

pub mod disturbance;
pub mod rw_unloading;
pub mod sensor_fov;
pub mod slew_time;
pub mod thrusters;
pub mod unload_time;
pub mod wheels;

pub use rw_unloading::{RwUnloading, RwUnloadingInput, RwUnloadingOutput};
pub use sensor_fov::{SensorFov, SensorFovInput, SensorFovOutput};
pub use slew_time::{SlewTime, SlewTimeInput, SlewTimeOutput};
pub use unload_time::{UnloadTime, UnloadTimeInput, UnloadTimeOutput};
