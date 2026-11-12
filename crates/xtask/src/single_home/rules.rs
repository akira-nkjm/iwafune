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
    // 次元 → 次元名（同じ名前の別物を作らない。別名を選ぶのは `dimension_label_preferring`）
    (
        "fn dimension_name(",
        &["crates/lib/units/src/format.rs"],
        "次元 → 次元名",
    ),
    // 位置の既定の単位（`fune_model::design::DEFAULT_POSITION_UNIT`・`position_unit`）
    (
        "unwrap_or(\"m\")",
        &["crates/app/model/src/design/part.rs"],
        "位置の既定の単位",
    ),
    // service の JSON Pointer の区間と、計画の変更をファイルの集合に当てる処理
    (
        "fn pointer_token",
        &["crates/app/service/src/analysis/dims.rs"],
        "JSON Pointer の区間の書き方",
    ),
    (
        "fn apply_changes",
        &["crates/app/store/src/write/mod.rs"],
        "変更の列をファイルの集合に当てる処理",
    ),
];

/// Rust のコードだけに適用する規則（同じ一覧の web/ の側は `WEB_RULES` が見る。テストの補助のデータだけは許す）。
pub(super) const RUST_RULES: &[(&str, &[&str], &str)] = &[
    // 役割の比較・対象は Role を渡す。綴りは role.rs の enum_text! だけ。
    (
        "by_role(ctx, \"",
        &["crates/app/model/src/role.rs"],
        "部品の役割（`Role`）",
    ),
    (
        "any_of_roles(&[\"",
        &["crates/app/model/src/role.rs"],
        "診断対象の役割（`Role`）",
    ),
    (
        "SENSOR_ROLES: &[&str]",
        &["crates/app/model/src/role.rs"],
        "センサの役割（`Role`）",
    ),
    (
        "fn system_of(",
        &["crates/app/model/src/part_system.rs"],
        "部品の系の決定",
    ),
    // 解析の系の一覧（`fune_model::design::AnalysisSystem`。`ALL`・`NAMES` を使う）
    (
        "\"power\", \"aocs\"",
        &["crates/app/model/src/design/analysis.rs"],
        "解析の系の一覧（`AnalysisSystem`）",
    ),
    // バジェットの種類の一覧（`BudgetKind::ALL`）
    (
        "\"mass\", \"power\", \"data\"",
        &["crates/app/model/src/design/defs.rs"],
        "バジェットの種類の一覧（`BudgetKind`）",
    ),
];

/// web/ の中だけに適用する規則（Rust 側の同名の関数を誤検出しないため）。
pub(super) const WEB_RULES: &[(&str, &[&str], &str)] = &[
    (
        "function systemTone(",
        &["web/src/lib/systems.ts"],
        "系の色",
    ),
    (
        "const SCENE_RATIOS",
        &["web/src/features/layout3d/scene.ts"],
        "3D の外形に対する表示寸法の比",
    ),
    (
        "function systemTree(",
        &["web/src/lib/design.ts"],
        "系別の表示ツリー",
    ),
    (
        "function parentOptions(",
        &["web/src/lib/design.ts"],
        "部品の置き場所の選択肢",
    ),
    ("Intl.NumberFormat", &["web/src/quantity/"], "数の表示"),
    // HTTP の呼び出しは API クライアントだけ
    ("fetch(", &["web/src/api/client.ts"], "API クライアント"),
    // 問い合わせの無効化は API 層だけ（編集の後に何を読み直すかを 1 か所で決める）
    ("invalidateQueries", &["web/src/api/"], "問い合わせの無効化"),
    // 数の桁の整形は量の表示だけ（単位と桁はサーバーの sat_units::format が選ぶ）
    ("toPrecision(", &["web/src/quantity/"], "数の整形"),
    ("toExponential(", &["web/src/quantity/"], "数の整形"),
    ("toFixed(", &["web/src/quantity/"], "数の整形"),
    // 読み込み中の表示は kit/feedback/Loading のスケルトンだけ（`Loading` で包み、画面読み上げにも伝える）
    (
        "@/ui/skeleton",
        &["web/src/kit/feedback/Loading.tsx"],
        "スケルトン（読み込み中の表示）",
    ),
    // 系とバジェットの種類の一覧と並びは API（`DefsView.systems`・`budget_kinds`）。Web に写しを書かない
    (
        "\"power\", \"aocs\"",
        &["web/src/test/"],
        "解析の系の一覧（API の `DefsView.systems`）",
    ),
    (
        "\"mass\", \"power\", \"data\"",
        &["web/src/test/"],
        "バジェットの種類の一覧（API の `DefsView.budget_kinds`）",
    ),
    // 負の字間は欧文を半角のつまった字にする（Noto Sans JP の欧文が詰まって読みにくい）。使わない
    ("tracking-tight", &["web/src/ui/"], "負の字間"),
    ("tracking-tighter", &["web/src/ui/"], "負の字間"),
    ("tracking-[-", &["web/src/ui/"], "負の字間"),
];

/// テスト以外のコードにだけ適用する規則（テストは手計算の値を直接書いてよい）。
pub(super) const PROD_RULES: &[(&str, &[&str], &str)] = &[
    (
        "\"off\"",
        &["crates/app/model/src/role.rs", "web/"],
        "慣例の停止状態（`STATE_OFF`）",
    ),
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
    // 参照の所在（`Design::references`）。テストは期待値として書いてよい
    (
        "catalog.as_deref() == Some(",
        &["crates/app/model/src/design/refs.rs"],
        "参照の索引",
    ),
    (
        "wire.as_deref() == Some(",
        &["crates/app/model/src/design/refs.rs"],
        "参照の索引",
    ),
];
