//! 「1 つの機能は 1 か所だけ」の機械的な検査（docs/architecture/registry.md §2 の一部を強制する）。
//!
//! 定義の書き出し（`fn rk4` や定数の値など）が、台帳で決めたファイル以外に現れたら違反にする。
//! 使う側（呼び出し）は検査しない。台帳に行を足したら、ここにも規則を足す。

use std::path::Path;

/// (探す文字列, 許すパスの接尾辞の一覧, 台帳での機能名)
const RULES: &[(&str, &[&str], &str)] = &[
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
    // 単位
    ("fn parse_unit", &["crates/lib/units/src/"], "単位の解析"),
    // 解析の登録
    (
        "fn run_json",
        &["crates/lib/analysis/src/catalog.rs"],
        "解析の登録",
    ),
];

/// web/ の中だけに適用する規則（Rust 側の同名の関数を誤検出しないため）。
const WEB_RULES: &[(&str, &[&str], &str)] = &[
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
const PROD_RULES: &[(&str, &[&str], &str)] = &[
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
        &[
            "crates/lib/astro/src/",
            // 球の地球での斜距離（registry: comm::link::slant_range_m）
            "crates/lib/analysis/src/comm/link.rs",
        ],
        "高度 → 半径の換算（`semi_major_axis_from_altitude`）",
    ),
];

/// 色を直書きしてよい web のファイル（配色の表とそのテスト）。
const HEX_COLOR_ALLOWED: &[&str] = &["web/src/lib/tone.ts", "web/src/lib/tone.test.ts"];

/// 違反を `errors` に積む。`files` はワークスペースのファイル一覧。
pub fn check(root: &Path, files: &[std::path::PathBuf], errors: &mut Vec<String>) {
    for file in files {
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if rel_s.starts_with("web/src/") && rel_s.ends_with(".css") {
            check_css_colors(&rel_s, file, errors);
            continue;
        }
        let is_code = rel_s.ends_with(".rs") || rel_s.ends_with(".ts") || rel_s.ends_with(".tsx");
        if !is_code || rel_s.starts_with("crates/xtask/") || rel_s.contains("/gen/") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let web = rel_s.starts_with("web/");
        let rules = if web {
            RULES.iter().chain(WEB_RULES)
        } else {
            RULES.iter().chain(&[][..])
        };
        for rule in rules {
            check_rule(&rel_s, &text, rule, errors);
        }
        if let Some(prod) = production_text(&rel_s, &text) {
            for rule in PROD_RULES {
                check_rule(&rel_s, prod, rule, errors);
            }
        }
        if web
            && !rel_s.contains(".test.")
            && !HEX_COLOR_ALLOWED.contains(&rel_s.as_str())
            && let Some(color) = find_hex_color(&text)
        {
            errors.push(format!(
                "{rel_s}: 色 `{color}` の直書きがあります。配色は {} だけに置きます（registry.md）",
                HEX_COLOR_ALLOWED[0]
            ));
        }
    }
}

/// CSS の色: 値は lib/tone.ts が正本で、CSS に書いてよいのは書き出した区間（`colors:begin` 〜 `colors:end`）だけ。
/// 区間の外に hex や rgb() / rgba() の色があれば違反にする。
fn check_css_colors(rel: &str, file: &Path, errors: &mut Vec<String>) {
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    let mut inside = false;
    for (n, line) in text.lines().enumerate() {
        if line.contains("colors:begin") {
            inside = true;
            continue;
        }
        if line.contains("colors:end") {
            inside = false;
            continue;
        }
        if inside {
            continue;
        }
        let rgb = line.contains("rgb(") || line.contains("rgba(");
        if let Some(color) = css_hex(line).or(rgb.then_some("rgb()")) {
            errors.push(format!(
                "{rel}:{}: 色 `{color}` の直書きがあります。色の値は {} に書き、pnpm -C web tokens で書き出します",
                n + 1,
                HEX_COLOR_ALLOWED[0]
            ));
        }
    }
}

fn check_rule(
    rel: &str,
    text: &str,
    (needle, allowed, what): &(&str, &[&str], &str),
    errors: &mut Vec<String>,
) {
    if text.contains(needle) && !allowed.iter().any(|a| is_allowed(rel, a)) {
        errors.push(format!(
            "{rel}: `{needle}` が見つかりました。{what}は {} だけに置きます（registry.md）",
            allowed.join(", ")
        ));
    }
}

/// テスト以外の部分（テストのファイルは None、それ以外は最初の `#[cfg(test)]` より前）。
fn production_text<'a>(rel: &str, text: &'a str) -> Option<&'a str> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let test_file = rel.contains("/tests/")
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name == "testkit.rs"
        || name.contains(".test.");
    if test_file {
        return None;
    }
    Some(text.split("#[cfg(test)]").next().unwrap_or(text))
}

/// 引用符で囲んだ `#rrggbb` か `#rgb`（CSS の色）を探す。
/// CSS の宣言の値に書いた hex の色（`#abc`・`#aabbcc` など。セレクタの `#id` は宣言の `:` より前なので見ない）
fn css_hex(line: &str) -> Option<&str> {
    let value = &line[line.find(':')? + 1..];
    for (i, _) in value.match_indices('#') {
        let digits = value[i + 1..]
            .bytes()
            .take_while(u8::is_ascii_hexdigit)
            .count();
        let after = value[i + 1 + digits..].bytes().next();
        let ends = after.is_none_or(|b| !b.is_ascii_alphanumeric() && b != b'_' && b != b'-');
        if matches!(digits, 3 | 4 | 6 | 8) && ends {
            return Some(&value[i..i + 1 + digits]);
        }
    }
    None
}

fn find_hex_color(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    for (i, _) in text.match_indices("\"#").chain(text.match_indices("'#")) {
        let start = i + 1;
        let digits = bytes[start + 1..]
            .iter()
            .take_while(|b| b.is_ascii_hexdigit())
            .count();
        let close = bytes.get(start + 1 + digits).copied();
        if (digits == 3 || digits == 6) && matches!(close, Some(b'"' | b'\'')) {
            return text.get(start..start + 1 + digits);
        }
    }
    None
}

/// `allowed` がファイル（`foo.rs`）なら、そのサブモジュールのディレクトリ（`foo/`）も許す。
fn is_allowed(path: &str, allowed: &str) -> bool {
    if path == allowed || path.starts_with(allowed) {
        return true;
    }
    allowed
        .strip_suffix(".rs")
        .is_some_and(|stem| path.starts_with(&format!("{stem}/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_display_conversions_are_rejected_only_in_production() {
        for needle in ["/ 3600.0", "* 100.0"] {
            let rule = PROD_RULES.iter().find(|r| r.0 == needle).unwrap();
            let mut errors = Vec::new();
            check_rule(
                "crates/lib/analysis/src/check.rs",
                needle,
                rule,
                &mut errors,
            );
            assert_eq!(errors.len(), 1);
            errors.clear();
            check_rule("crates/lib/units/src/format.rs", needle, rule, &mut errors);
            assert!(errors.is_empty());
            let code = format!("fn run() {{}}\n#[cfg(test)]\nmod tests {{ {needle} }}");
            check_rule(
                "crates/lib/analysis/src/check.rs",
                production_text("check.rs", &code).unwrap(),
                rule,
                &mut errors,
            );
            assert!(errors.is_empty());
        }
    }
}
