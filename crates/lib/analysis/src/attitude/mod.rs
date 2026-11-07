//! 姿勢系の解析。各解析は 1 ファイル 1 つ（入出力の型と実装）。系の中で共有する計算はサブモジュールに置き、台帳に登録する。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`disturbance`] | 擾乱トルク（重力傾斜・空力・太陽輻射圧・残留磁気） |
//! | [`wheels`] | ホイールの組（任意の台数・配置）への擬似逆行列での配分と容量 |
//! | [`rw_unloading`] | `attitude.rw_unloading` RW アンロード頻度 |
//! | [`unload_time`] | `attitude.unload_time` MTQ・RCS でのアンロード時間 |
//! | [`sensor_fov`] | `attitude.sensor_fov` センサの禁止角への侵入 |

pub mod disturbance;
pub mod rw_unloading;
pub mod sensor_fov;
pub mod unload_time;
pub mod wheels;

pub use rw_unloading::{RwUnloading, RwUnloadingInput, RwUnloadingOutput};
pub use sensor_fov::{SensorFov, SensorFovInput, SensorFovOutput};
pub use unload_time::{UnloadTime, UnloadTimeInput, UnloadTimeOutput};
