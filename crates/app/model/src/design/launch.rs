//! 打ち上げ機とディスペンサー（種別 `launcher`・`dispenser` のカタログ品）の選択と、その属性の名前。
//!
//! 選ぶのは `conditions/launch.toml` の `launcher`・`dispenser`（部品の `catalog` と同じ `<カタログ>/<ID>` の形で、読み方は
//! [`crate::path::split_catalog_ref`] のまま）。**衛星の部品ではない**ので、部品・バジェット・図には入らない。
//! 属性の値を SI にするのは [`crate::resolve::resolve_catalog_item`]（部品の値と同じ）で、どの属性をどの解析が使うかは projection が決める。

use super::{CatalogItem, Design};
use crate::enum_text::enum_text;
use serde::{Deserialize, Serialize};

enum_text! {
    /// 打ち上げ条件が選ぶ品の位置づけ。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum LaunchInterface: "launch interface" {
        /// 打ち上げ機
        Launcher = "launcher",
        /// ディスペンサー（打ち上げ機と衛星の間の接点）
        Dispenser = "dispenser",
    }
}

impl LaunchInterface {
    /// 先に使う順（衛星に近い接点から）。**同じ属性を両方が持つときの優先はこの順**。
    pub const NEAREST_FIRST: [Self; 2] = [Self::Dispenser, Self::Launcher];

    /// この位置づけの品の種別 ID（組み込みの種別。`conditions/launch.toml` の欄名と同じ綴り）。
    pub fn kind_id(self) -> &'static str {
        self.as_str()
    }
}

/// 打ち上げ機・ディスペンサーの属性 ID（組み込みの属性定義 `builtin/attributes.toml` の綴り）。
pub mod attr {
    /// 搭載質量の上限（質量）
    pub const MAX_PAYLOAD_MASS: &str = "max_payload_mass";
    /// 収納外形の許容（箱。機体座標の x, y, z）
    pub const ENVELOPE: &str = "launch_envelope";
    /// 準静的荷重（機軸方向。`g0`）
    pub const LOAD_AXIAL: &str = "launch_load_axial";
    /// 準静的荷重（直交方向。`g0`）
    pub const LOAD_LATERAL: &str = "launch_load_lateral";
    /// 最低固有振動数（機軸方向）
    pub const MIN_FREQUENCY_AXIAL: &str = "min_frequency_axial";
    /// 最低固有振動数（直交方向）
    pub const MIN_FREQUENCY_LATERAL: &str = "min_frequency_lateral";
    /// 分離時の角速度の上限（チップオフ）
    pub const MAX_TIP_OFF_RATE: &str = "max_tip_off_rate";
    /// すべて（組み込みの種別の属性と、定義のテストの照合に使う）
    pub const ALL: [&str; 7] = [
        MAX_PAYLOAD_MASS,
        ENVELOPE,
        LOAD_AXIAL,
        LOAD_LATERAL,
        MIN_FREQUENCY_AXIAL,
        MIN_FREQUENCY_LATERAL,
        MAX_TIP_OFF_RATE,
    ];
}

impl Design {
    /// `conditions/launch.toml` が選んでいる品の参照（書かれていなければ `None`）。
    pub fn launch_selection(&self, which: LaunchInterface) -> Option<&str> {
        let launch = self.conditions.launch.as_ref()?;
        match which {
            LaunchInterface::Launcher => launch.launcher.as_deref(),
            LaunchInterface::Dispenser => launch.dispenser.as_deref(),
        }
    }

    /// 選んでいる品（参照が定義されていなければ `None`）。
    pub fn launch_item(&self, which: LaunchInterface) -> Option<&CatalogItem> {
        self.catalog.get(self.launch_selection(which)?)
    }

    /// 種別が打ち上げ機・ディスペンサーの種別か（継承を含む）。部品の種別にはできない。
    pub fn is_launch_kind(&self, kind: &str) -> bool {
        self.kind_chain(kind).iter().any(|k| {
            LaunchInterface::NEAREST_FIRST
                .iter()
                .any(|w| k.id.as_str() == w.kind_id())
        })
    }

    /// 種別が `which` の種別か（継承を含む。選べる品の種別の検査）。
    pub fn is_launch_kind_of(&self, kind: &str, which: LaunchInterface) -> bool {
        self.kind_chain(kind)
            .iter()
            .any(|k| k.id.as_str() == which.kind_id())
    }
}
