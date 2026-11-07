//! 単位式の表示用の綴り（`mN*m*s` → `mN·m·s`、`m^2` → `m²`、`degC` → `°C`）。
//! 表示の文字列だけに使い、解析できる式（[`super::FormattedQuantity::unit`]）は変えない。

use alloc::string::String;

/// マイクロ（`u`）の後に続けば `µ` に直す記号。
const MICRO_BASES: &[&str] = &[
    "m", "s", "g", "N", "T", "W", "A", "V", "J", "F", "H", "Pa", "Gy", "Hz", "rad", "sr",
];

/// 単位式を読みやすい綴りにする。`*` は中黒、整数の冪は上付き、`deg`・`degC`・`degF`・`ohm` は記号、
/// マイクロの `u` は `µ` にする。括弧付きの分数冪（`Hz^(1/2)`）はそのまま。
pub fn unit_label(unit: &str) -> String {
    let mut out = String::with_capacity(unit.len() + 4);
    let mut chars = unit.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '*' => out.push('·'),
            '^' if chars
                .peek()
                .is_some_and(|(_, n)| *n == '-' || n.is_ascii_digit()) =>
            {
                while let Some(&(_, n)) = chars.peek() {
                    match superscript(n) {
                        Some(s) => out.push(s),
                        None => break,
                    }
                    chars.next();
                }
            }
            c if c.is_ascii_alphabetic() => {
                let mut end = i + c.len_utf8();
                while let Some(&(j, n)) = chars.peek() {
                    if !n.is_ascii_alphabetic() {
                        break;
                    }
                    end = j + n.len_utf8();
                    chars.next();
                }
                push_symbol(&mut out, &unit[i..end]);
            }
            c => out.push(c),
        }
    }
    out
}

fn push_symbol(out: &mut String, token: &str) {
    match token {
        "deg" => out.push('°'),
        "degC" => out.push_str("°C"),
        "degF" => out.push_str("°F"),
        "ohm" => out.push('Ω'),
        t if t.len() > 1 && t.starts_with('u') && MICRO_BASES.contains(&&t[1..]) => {
            out.push('µ');
            out.push_str(&t[1..]);
        }
        t => out.push_str(t),
    }
}

fn superscript(c: char) -> Option<char> {
    Some(match c {
        '-' => '⁻',
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::unit_label;

    #[test]
    fn labels_read_like_print() {
        assert_eq!(unit_label("mN*m*s"), "mN·m·s");
        assert_eq!(unit_label("kg*m^2"), "kg·m²");
        assert_eq!(unit_label("m/s^2"), "m/s²");
        assert_eq!(unit_label("1/s^-1"), "1/s⁻¹");
        assert_eq!(unit_label("degC"), "°C");
        assert_eq!(unit_label("deg/h^(1/2)"), "°/h^(1/2)");
        assert_eq!(unit_label("J/(kg*K)"), "J/(kg·K)");
        assert_eq!(unit_label("um"), "µm");
        assert_eq!(unit_label("uT"), "µT");
        assert_eq!(unit_label("ohm"), "Ω");
        assert_eq!(unit_label("mWh"), "mWh");
        assert_eq!(unit_label("µN*m"), "µN·m");
        assert_eq!(unit_label(""), "");
    }
}
