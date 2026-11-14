//! 「1 か所だけ」の規則の表（データだけ。照合は親の `single_home`）。台帳（registry.md §2）に行を足したら、ここにも足す。

/// (探す文字列, 許すパスの接尾辞の一覧, 台帳での機能名)
pub(super) type Rule = (&'static str, &'static [&'static str], &'static str);

mod app;
mod compute;

/// Rust と web/ の両方に適用する規則（分野ごとのファイル）。
pub(super) const SHARED: &[&[Rule]] = &[compute::RULES, app::RULES];

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
        &[
            "crates/app/model/src/role.rs",
            "crates/lib/analysis/src/modes.rs",
            "web/",
        ],
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
