//! `major.minor.patch` の版の解析と比較（リリースのタグ・`--version`・このバイナリの版で共通）。

use crate::error::{Result, ServiceError};
use std::fmt;

/// 版（`major.minor.patch`。数としての比較）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    /// `0.5.1` または `v0.5.1` を読む。それ以外（`1.2`・`1.2.3-rc1`・空）は誤り。
    pub fn parse(text: &str) -> Result<Self> {
        let bad = || {
            ServiceError::InvalidInput(format!(
                "Unsupported version '{text}' (expected major.minor.patch, such as 0.5.2)"
            ))
        };
        let body = text.strip_prefix('v').unwrap_or(text);
        let mut parts = body.split('.');
        let mut next = || -> Result<u64> {
            let p = parts.next().ok_or_else(bad)?;
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad());
            }
            p.parse().map_err(|_| bad())
        };
        let version = Self {
            major: next()?,
            minor: next()?,
            patch: next()?,
        };
        if parts.next().is_some() {
            return Err(bad());
        }
        Ok(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_compares_numerically() {
        let a = Version::parse("0.5.2").unwrap();
        assert_eq!(a, Version::parse("v0.5.2").unwrap());
        assert!(Version::parse("0.10.0").unwrap() > Version::parse("0.9.9").unwrap());
        assert!(Version::parse("0.5.10").unwrap() > a);
        assert_eq!(a.to_string(), "0.5.2");
    }

    #[test]
    fn rejects_anything_but_three_numbers() {
        for bad in [
            "",
            "1.2",
            "1.2.3.4",
            "1.2.3-rc1",
            "a.b.c",
            "1..3",
            "vv1.2.3",
            "+1.2.3",
            "1.2.3 ",
        ] {
            assert!(Version::parse(bad).is_err(), "{bad}");
        }
    }
}
