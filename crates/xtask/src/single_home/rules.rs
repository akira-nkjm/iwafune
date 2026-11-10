//! 「1 か所だけ」の規則の表（データだけ。照合は親の `single_home`）。台帳（registry.md §2）に行を足したら、ここにも足す。

/// (探す文字列, 許すパスの接尾辞の一覧, 台帳での機能名)
pub(super) const RULES: &[(&str, &[&str], &str)] = &[
    // 積分器
    ("fn rk4", &["crates/lib/astro/src/ode.rs"], "積分器"),
    ("struct Dop853", &["crates/lib/astro/src/ode.rs"], "積分器"),
    // 定数（値の手書き）
    (
        "3.986_004_418e14",
        &["crates/lib/astro/src/constants.rs"],
        "地球の μ",
    ),
    (
        "3.986004418e14",
        &["crates/lib/astro/src/constants.rs"],
        "地球の μ",
    ),
    (
        "398600.4418",
        &["crates/lib/astro/src/constants.rs"],
        "地球の μ",
    ),
    (
        "6_378_137",
        &["crates/lib/astro/src/constants.rs"],
        "地球の赤道半径",
    ),
    (
        "6378137",
        &["crates/lib/astro/src/constants.rs"],
        "地球の赤道半径",
    ),
    (
        "5.670_374_419e-8",
        &["crates/lib/astro/src/constants.rs"],
        "シュテファン＝ボルツマン定数",
    ),
    (
        "5.670374419e-8",
        &["crates/lib/astro/src/constants.rs"],
        "シュテファン＝ボルツマン定数",
    ),
    (
        "1.380_649e-23",
        &["crates/lib/astro/src/constants.rs"],
        "ボルツマン定数",
    ),
    (
        "1.380649e-23",
        &["crates/lib/astro/src/constants.rs"],
        "ボルツマン定数",
    ),
    (
        "2.997_924_58e8",
        &["crates/lib/astro/src/constants.rs"],
        "光速",
    ),
    ("299792458", &["crates/lib/astro/src/constants.rs"], "光速"),
    (
        "fn solve_dense",
        &["crates/lib/astro/src/math"],
        "連立一次方程式の解法",
    ),
    (
        "fn gauss_elim",
        &["crates/lib/astro/src/math"],
        "連立一次方程式の解法",
    ),
    // 天体・軌道・環境の計算
    (
        "fn earth_shadow_factor",
        &["crates/lib/astro/src/shadow.rs"],
        "地球の影",
    ),
    (
        "fn sun_direction",
        &["crates/lib/astro/src/ephem.rs"],
        "太陽の方向",
    ),
    ("fn gmst", &["crates/lib/astro/src/time.rs"], "恒星時"),
    (
        "fn eci_to_ecef",
        &["crates/lib/astro/src/earth.rs"],
        "座標変換",
    ),
    (
        "fn dipole_field",
        &["crates/lib/astro/src/geomag.rs"],
        "地磁気",
    ),
    (
        "fn density_exponential",
        &["crates/lib/astro/src/atmosphere.rs"],
        "大気密度",
    ),
    (
        "fn look_angles",
        &["crates/lib/astro/src/ground.rs"],
        "見通し角",
    ),
    (
        "fn find_passes",
        &["crates/lib/astro/src/ground.rs"],
        "パスの検出",
    ),
    (
        "fn solar_incidence",
        &["crates/lib/astro/src/illumination.rs"],
        "面への太陽入射",
    ),
    (
        "fn body_to_eci",
        &["crates/lib/astro/src/attitude.rs"],
        "姿勢則",
    ),
    (
        "fn sso_inclination",
        &["crates/lib/astro/src/orbit.rs"],
        "太陽同期傾斜角",
    ),
    ("fn beta_angle", &["crates/lib/astro/src/orbit.rs"], "β 角"),
    // 区間の抽出
    (
        "fn windows(",
        &["crates/lib/astro/src/timeline.rs"],
        "時系列の区間の抽出",
    ),
    (
        "fn orbit_ranges(",
        &["crates/lib/analysis/src/spec.rs"],
        "周回の切り出し",
    ),
    // 時系列の集計（spec の子 `spec/orbit.rs` も spec.rs の許可に入る）
    (
        "fn orbit_mean(",
        &["crates/lib/analysis/src/spec.rs"],
        "周回平均",
    ),
    (
        "fn time_fraction(",
        &["crates/lib/analysis/src/spec.rs"],
        "時間の割合",
    ),
    (
        "SUNLIT_THRESHOLD: f64",
        &["crates/lib/analysis/src/spec.rs"],
        "日照の閾値",
    ),
    (
        "fn slant_range",
        &["crates/lib/astro/src/ground.rs"],
        "斜距離",
    ),
    ("fn drag_force", &["crates/lib/astro/src/force.rs"], "抗力"),
    (
        "struct StateValues",
        &["crates/lib/analysis/src/modes.rs"],
        "状態ごとの値の表",
    ),
    (
        "fn range_verdict",
        &["crates/lib/analysis/src/thermal/range.rs"],
        "温度範囲の判定",
    ),
    // 単位
    ("fn parse_unit", &["crates/lib/units/src/"], "単位の解析"),
    // 解析の登録
    (
        "fn run_json",
        &["crates/lib/analysis/src/catalog.rs"],
        "解析の登録",
    ),
    // 列挙の綴り（serde_json を経由して綴りを得る関数を書かない。`enum_text!` の `as_str` を使う）
    (
        "fn enum_str",
        &["crates/app/model/src/enum_text.rs"],
        "列挙の綴り（`enum_text!`）",
    ),
    // 値の文字列の書式（読む向きと書く向き）
    (
        "fn value_to_text",
        &["crates/app/model/src/value/text"],
        "値の文字列の書き方",
    ),
    (
        "fn setting_to_text",
        &["crates/app/model/src/value/text"],
        "設定の値の文字列の書き方",
    ),
    (
        "fn list_from_text",
        &["crates/app/model/src/value/text.rs"],
        "一覧の読み方",
    ),
];

/// web/ の中だけに適用する規則（Rust 側の同名の関数を誤検出しないため）。
pub(super) const WEB_RULES: &[(&str, &[&str], &str)] = &[
    ("Intl.NumberFormat", &["web/src/quantity/"], "数の表示"),
    // HTTP の呼び出しは API クライアントだけ
    ("fetch(", &["web/src/api/client.ts"], "API クライアント"),
    // 問い合わせの無効化は API 層だけ（編集の後に何を読み直すかを 1 か所で決める）
    ("invalidateQueries", &["web/src/api/"], "問い合わせの無効化"),
    // 数の桁の整形は量の表示だけ（単位と桁はサーバーの sat_units::format が選ぶ）
    ("toPrecision(", &["web/src/quantity/"], "数の整形"),
    ("toExponential(", &["web/src/quantity/"], "数の整形"),
    ("toFixed(", &["web/src/quantity/"], "数の整形"),
    // 負の字間は欧文を半角のつまった字にする（Noto Sans JP の欧文が詰まって読みにくい）。使わない
    ("tracking-tight", &["web/src/ui/"], "負の字間"),
    ("tracking-tighter", &["web/src/ui/"], "負の字間"),
    ("tracking-[-", &["web/src/ui/"], "負の字間"),
];

/// テスト以外のコードにだけ適用する規則（テストは手計算の値を直接書いてよい）。
pub(super) const PROD_RULES: &[(&str, &[&str], &str)] = &[
    // 解析の表示換算を禁止する。現在の production の出現は全て判定文の換算。
    (
        "/ 3600.0",
        &[
            "crates/lib/units/",
            "crates/lib/astro/",
            "crates/app/",
            "web/",
        ],
        "J → Wh の表示換算（sat-units）",
    ),
    (
        "* 100.0",
        &[
            "crates/lib/units/",
            "crates/lib/astro/",
            "crates/app/",
            "web/",
        ],
        "比 → % の表示換算（sat-units）",
    ),
    ("fn to_db", &["crates/lib/units/src/"], "dB の換算"),
    // 常用対数は dB の換算（`sat_units::ratio_to_db`）にだけ使う。テストは手計算の確かめに使ってよい
    (".log10()", &["crates/lib/units/"], "dB の換算（常用対数）"),
    (
        "86_400",
        &[
            "crates/lib/astro/src/constants.rs",
            "crates/lib/astro/src/time.rs",
            "crates/lib/units/src/",
        ],
        "1 日の秒数（`SECONDS_PER_DAY`）",
    ),
    (
        "86400.0",
        &["crates/lib/astro/src/constants.rs"],
        "1 日の秒数（`SECONDS_PER_DAY`）",
    ),
    (
        "R_EARTH +",
        &["crates/lib/astro/src/"],
        "高度 → 半径の換算（`semi_major_axis_from_altitude`）",
    ),
];
