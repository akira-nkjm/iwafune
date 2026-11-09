//! ファイルと Op の JSON の上でのポートの参照の書き方（`"PCDU.OUT3"` の文字列）。`#[serde(with = "port_text")]` で使う。
//!
//! 読みは [`PortRef`] の `FromStr`（読めなければ読み込みの誤り）、書きは `Display` だけ。表示用の DTO は構造（`{ part, port }`）のまま。

use super::PortRef;
use serde::{Deserialize, Deserializer, Serializer, de::Error};

/// `"PART.PORT"` で書く。
pub fn serialize<S: Serializer>(port: &PortRef, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(port)
}

/// `"PART.PORT"` を読む。
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PortRef, D::Error> {
    String::deserialize(d)?.parse().map_err(D::Error::custom)
}

/// `Option<PortRef>` の版（無ければ書かない欄）。
pub mod option {
    use super::PortRef;
    use serde::{Deserialize, Deserializer, Serializer};

    /// `Some` なら `"PART.PORT"`。
    pub fn serialize<S: Serializer>(port: &Option<PortRef>, s: S) -> Result<S::Ok, S::Error> {
        match port {
            Some(p) => s.collect_str(p),
            None => s.serialize_none(),
        }
    }

    /// 文字列か null を読む。
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<PortRef>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|text| text.parse().map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::PortRef;
    use serde::{Deserialize, Serialize};

    #[test]
    fn port_refs_are_text_in_files() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Doc {
            #[serde(with = "super")]
            from: PortRef,
            #[serde(default, with = "super::option")]
            source: Option<PortRef>,
        }
        let doc: Doc = toml::from_str("from = \"PCDU.OUT3\"\n").unwrap();
        assert_eq!(doc.from.port, "OUT3");
        assert_eq!(doc.source, None);
        let text = toml::to_string(&Doc {
            source: Some(doc.from.clone()),
            ..doc
        })
        .unwrap();
        assert_eq!(text, "from = \"PCDU.OUT3\"\nsource = \"PCDU.OUT3\"\n");
        let err = toml::from_str::<Doc>("from = \"PCDU\"\n").unwrap_err();
        assert!(err.message().contains("部品.ポート"), "{err}");
    }
}
