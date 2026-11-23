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
        "part roles (`Role`)",
    ),
    (
        "any_of_roles(&[\"",
        &["crates/app/model/src/role.rs"],
        "roles of diagnosed parts (`Role`)",
    ),
    (
        "SENSOR_ROLES: &[&str]",
        &["crates/app/model/src/role.rs"],
        "sensor roles (`Role`)",
    ),
    (
        "fn system_of(",
        &["crates/app/model/src/part_system.rs"],
        "deciding the system of a part",
    ),
    // 解析の系の一覧（`fune_model::design::AnalysisSystem`。`ALL`・`NAMES` を使う）
    (
        "\"aocs\", \"cdh\"",
        &["crates/app/model/src/design/analysis.rs"],
        "the list of analysis systems (`AnalysisSystem`)",
    ),
    // 並びを変える前の綴り（古い写しが紛れ込んでも見つける）
    (
        "\"power\", \"aocs\"",
        &["crates/app/model/src/design/analysis.rs"],
        "the list of analysis systems (`AnalysisSystem`)",
    ),
    // バジェットの種類の一覧（`BudgetKind::ALL`）
    (
        "\"mass\", \"power\", \"data\"",
        &["crates/app/model/src/design/defs.rs"],
        "the list of budget kinds (`BudgetKind`)",
    ),
];

/// web/ の中だけに適用する規則（Rust 側の同名の関数を誤検出しないため）。
pub(super) const WEB_RULES: &[(&str, &[&str], &str)] = &[
    (
        "function systemTone(",
        &["web/src/lib/systems.ts"],
        "system colours",
    ),
    (
        "const SCENE_RATIOS",
        &["web/src/features/layout3d/scene.ts"],
        "the display size ratio against the 3D outline",
    ),
    (
        "function systemTree(",
        &["web/src/lib/design.ts"],
        "the per-system display tree",
    ),
    (
        "function parentOptions(",
        &["web/src/lib/design.ts"],
        "the choices for where a part is placed",
    ),
    (
        "Intl.NumberFormat",
        &["web/src/quantity/"],
        "number display",
    ),
    // HTTP の呼び出しは API クライアントだけ
    ("fetch(", &["web/src/api/client.ts"], "the API client"),
    // 問い合わせの無効化は API 層だけ（編集の後に何を読み直すかを 1 か所で決める）
    ("invalidateQueries", &["web/src/api/"], "query invalidation"),
    // 数の桁の整形は量の表示だけ（単位と桁はサーバーの sat_units::format が選ぶ）
    ("toPrecision(", &["web/src/quantity/"], "number formatting"),
    (
        "toExponential(",
        &["web/src/quantity/"],
        "number formatting",
    ),
    ("toFixed(", &["web/src/quantity/"], "number formatting"),
    // 読み込み中の表示は kit/feedback/Loading のスケルトンだけ（`Loading` で包み、画面読み上げにも伝える）
    (
        "@/ui/skeleton",
        &["web/src/kit/feedback/Loading.tsx"],
        "skeletons (loading display)",
    ),
    // 待ちを示す動くロゴは kit/feedback の部品だけが定義し、使う画面は `Working` / `Refreshing` / `Refreshable` を通す
    (
        "MarkLoader",
        &["web/src/kit/feedback/"],
        "the animated logo mark for waiting (loading display)",
    ),
    (
        "mark-glow",
        &["web/src/kit/feedback/"],
        "the animated logo mark for waiting (loading display)",
    ),
    // 系とバジェットの種類の一覧と並びは API（`DefsView.systems`・`budget_kinds`）。Web に写しを書かない
    (
        "\"aocs\", \"cdh\"",
        &["web/src/test/"],
        "the list of analysis systems (the API's `DefsView.systems`)",
    ),
    (
        "\"power\", \"aocs\"",
        &["web/src/test/"],
        "the list of analysis systems (the API's `DefsView.systems`)",
    ),
    (
        "\"mass\", \"power\", \"data\"",
        &["web/src/test/"],
        "the list of budget kinds (the API's `DefsView.budget_kinds`)",
    ),
    // バジェットの種類を JSX の属性に直書きしない（`<BudgetPanel kind="mass" />`）。概要は `DefsView.budget_kinds` を回す
    (
        "kind=\"mass\"",
        &["web/src/test/"],
        "a budget kind written in JSX (iterate the API's `DefsView.budget_kinds`)",
    ),
    (
        "kind=\"power\"",
        &["web/src/test/"],
        "a budget kind written in JSX (iterate the API's `DefsView.budget_kinds`)",
    ),
    (
        "kind=\"data\"",
        &["web/src/test/"],
        "a budget kind written in JSX (iterate the API's `DefsView.budget_kinds`)",
    ),
    // 負の字間は欧文を半角のつまった字にする（Noto Sans JP の欧文が詰まって読みにくい）。使わない
    (
        "tracking-tight",
        &["web/src/ui/"],
        "negative letter-spacing",
    ),
    (
        "tracking-tighter",
        &["web/src/ui/"],
        "negative letter-spacing",
    ),
    ("tracking-[-", &["web/src/ui/"], "negative letter-spacing"),
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
        "the conventional off state (`STATE_OFF`)",
    ),
    // 解析の表示換算を禁止する。J → Wh の換算は sat-units だけ（nrlmsise00 の `/ 3600.0` は時刻の秒 → 時で、表示の換算ではない）
    (
        "/ 3600.0",
        &[
            "crates/lib/units/",
            "crates/lib/astro/src/atmosphere/nrlmsise00.rs",
        ],
        "the J to Wh display conversion (sat-units)",
    ),
    // 比 → % の換算は sat-units だけ。概要のメーターの位置（0〜100）は表示の換算ではなく目盛りの値で、API に移す（#328）
    (
        "* 100.0",
        &[
            "crates/lib/units/",
            "crates/app/service/src/analysis/overview/power/meter.rs",
        ],
        "the ratio to % display conversion (sat-units)",
    ),
    // 推薬の流量 F / (Isp · g₀) は `sat_astro::rocket::mass_flow_kg_s`（排気速度は `exhaust_velocity`）だけ
    (
        "isp * G0",
        &["crates/lib/astro/src/rocket.rs"],
        "propellant mass flow and exhaust velocity (`sat_astro::rocket`)",
    ),
    (
        "isp_s * G0",
        &["crates/lib/astro/src/rocket.rs"],
        "propellant mass flow and exhaust velocity (`sat_astro::rocket`)",
    ),
    (
        "G0 * isp",
        &["crates/lib/astro/src/rocket.rs"],
        "propellant mass flow and exhaust velocity (`sat_astro::rocket`)",
    ),
    (
        "/ exhaust_velocity(",
        &["crates/lib/astro/src/rocket.rs"],
        "propellant mass flow (`sat_astro::rocket::mass_flow_kg_s`)",
    ),
    (
        "fn ratio_to_db",
        &["crates/lib/units/src/"],
        "dB conversion",
    ),
    // 常用対数は dB の換算（`sat_units::ratio_to_db`）にだけ使う。テストは手計算の確かめに使ってよい
    (
        ".log10()",
        &["crates/lib/units/"],
        "dB conversion (common logarithm)",
    ),
    (
        "86_400",
        &[
            "crates/lib/astro/src/constants.rs",
            "crates/lib/astro/src/time.rs",
            "crates/lib/units/src/",
        ],
        "seconds per day (`SECONDS_PER_DAY`)",
    ),
    (
        "86400.0",
        &["crates/lib/astro/src/constants.rs"],
        "seconds per day (`SECONDS_PER_DAY`)",
    ),
    (
        "R_EARTH +",
        &["crates/lib/astro/src/"],
        "the altitude to radius conversion (`semi_major_axis_from_altitude`)",
    ),
    // 読み込み中のパルスのアニメーションは ui のスケルトンだけ（画面は `kit/feedback` の部品を通す）
    (
        "animate-pulse",
        &["web/src/ui/skeleton.tsx"],
        "the pulse animation of skeletons (loading display)",
    ),
    // バジェットの種類を引数に直書きしない（概要は `DefsView.budget_kinds` を回す）。質量カードの見出しの選び方は API に移す（#328）
    (
        "budget(\"mass\")",
        &["web/src/features/overview/cards.ts"],
        "a budget kind written as an argument (iterate the API's `DefsView.budget_kinds`)",
    ),
    // バジェットの種類を引数に直書きしない（概要は `DefsView.budget_kinds` を回す）。質量カードの見出しの選び方は API に移す（#328）
    (
        "budget(\"power\")",
        &["web/src/features/overview/cards.ts"],
        "a budget kind written as an argument (iterate the API's `DefsView.budget_kinds`)",
    ),
    // バジェットの種類を引数に直書きしない（概要は `DefsView.budget_kinds` を回す）。質量カードの見出しの選び方は API に移す（#328）
    (
        "budget(\"data\")",
        &["web/src/features/overview/cards.ts"],
        "a budget kind written as an argument (iterate the API's `DefsView.budget_kinds`)",
    ),
    // バジェットの種類を引数に直書きしない（概要は `DefsView.budget_kinds` を回す）。質量カードの見出しの選び方は API に移す（#328）
    (
        "budget('mass')",
        &["web/src/features/overview/cards.ts"],
        "a budget kind written as an argument (iterate the API's `DefsView.budget_kinds`)",
    ),
    // バジェットの種類を引数に直書きしない（概要は `DefsView.budget_kinds` を回す）。質量カードの見出しの選び方は API に移す（#328）
    (
        "budget('power')",
        &["web/src/features/overview/cards.ts"],
        "a budget kind written as an argument (iterate the API's `DefsView.budget_kinds`)",
    ),
    // バジェットの種類を引数に直書きしない（概要は `DefsView.budget_kinds` を回す）。質量カードの見出しの選び方は API に移す（#328）
    (
        "budget('data')",
        &["web/src/features/overview/cards.ts"],
        "a budget kind written as an argument (iterate the API's `DefsView.budget_kinds`)",
    ),
    // 問い合わせのキーは API 層のキー表（keys.ts）と、まとめて問い合わせる層・待ちの部品だけが組み立てる
    (
        "queryKey: [",
        &[
            "web/src/api/keys.ts",
            "web/src/quantity/format-batch.ts",
            "web/src/kit/feedback/Settled.tsx",
        ],
        "query keys (`web/src/api/keys.ts`)",
    ),
    // 参照の所在（`Design::references`）。テストは期待値として書いてよい
    (
        "catalog.as_deref() == Some(",
        &["crates/app/model/src/design/refs.rs"],
        "the reference index",
    ),
    (
        "wire.as_deref() == Some(",
        &["crates/app/model/src/design/refs.rs"],
        "the reference index",
    ),
];
