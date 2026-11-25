//! Gauss 係数の配布ファイル（文字列形式）の読み書き。形式は docs/architecture.md の
//! 「Geomagnetic field coefficients」にある。I/O はしない（バイト列を受け取る）。

use super::gauss::{GaussModel, GaussTerm};
use crate::AstroError;
use crate::time::Epoch;

fn bad(line_no: usize, msg: &str) -> AstroError {
    AstroError::InvalidInput(format!(
        "Geomagnetic coefficient file, line {line_no}: {msg}"
    ))
}

impl GaussModel {
    /// 文字列形式のバイト列（UTF-8）から読む。
    ///
    /// 空行と `#` 以降はコメント。`キー: 値` の行が見出し（`name`・`epoch`・`radius_km`。`epoch` と `radius_km` は必須）、
    /// それ以外の行が係数 `n m g h [g_dot h_dot]`（空白区切り。nT と nT/年。年変化は省略すると 0）。
    pub fn from_text_bytes(bytes: &[u8]) -> Result<Self, AstroError> {
        let text = std::str::from_utf8(bytes).map_err(|_| {
            AstroError::InvalidInput("Geomagnetic coefficient file is not UTF-8".into())
        })?;
        let (mut name, mut epoch, mut radius_km) = (None, None, None);
        let mut terms = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line_no = i + 1;
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                let value = value.trim();
                match key.trim() {
                    "name" => name = Some(value.to_string()),
                    "epoch" => {
                        epoch = Some(Epoch::parse(value).map_err(|_| bad(line_no, "bad epoch"))?)
                    }
                    "radius_km" => {
                        radius_km = Some(
                            value
                                .parse::<f64>()
                                .map_err(|_| bad(line_no, "bad radius_km"))?,
                        );
                    }
                    other => return Err(bad(line_no, &format!("unknown header key `{other}`"))),
                }
                continue;
            }
            terms.push(parse_term(line, line_no)?);
        }
        let epoch = epoch.ok_or_else(|| bad(0, "missing `epoch`"))?;
        let radius_km = radius_km.ok_or_else(|| bad(0, "missing `radius_km`"))?;
        Self::new(
            name.as_deref().unwrap_or("unnamed"),
            epoch,
            radius_km * 1e3,
            terms,
        )
    }

    /// 文字列形式に書き出す（[`GaussModel::from_text_bytes`] で元に戻る）。
    pub fn to_text(&self) -> String {
        let mut out = format!(
            "name: {}\nepoch: {}\nradius_km: {}\n# n m g h g_dot h_dot  (nT, nT/yr)\n",
            self.name(),
            self.epoch().to_iso(),
            self.ref_radius_m() / 1e3
        );
        for t in self.terms() {
            out.push_str(&format!(
                "{} {} {} {} {} {}\n",
                t.n, t.m, t.g, t.h, t.g_dot, t.h_dot
            ));
        }
        out
    }
}

fn parse_term(line: &str, line_no: usize) -> Result<GaussTerm, AstroError> {
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() != 4 && cols.len() != 6 {
        return Err(bad(line_no, "expected `n m g h [g_dot h_dot]`"));
    }
    let int = |s: &str| {
        s.parse::<u8>()
            .map_err(|_| bad(line_no, "bad degree or order"))
    };
    let num = |s: &str| s.parse::<f64>().map_err(|_| bad(line_no, "bad number"));
    let at = |k: usize| cols.get(k).map_or(Ok(0.0), |s| num(s));
    Ok(GaussTerm {
        n: int(cols[0])?,
        m: int(cols[1])?,
        g: num(cols[2])?,
        h: num(cols[3])?,
        g_dot: at(4)?,
        h_dot: at(5)?,
    })
}
