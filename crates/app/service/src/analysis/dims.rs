//! 解析の出力の量の次元: 出力を型付きのまま書き出し、`Qty` の葉ごとに JSON Pointer → 次元名を集める。
//! `Qty::serialize` が書き出す直前に記録する次元（`sat_units::take_last_dimension`）を、数値の葉の位置と結び付けるだけ。
//! 出力の JSON そのものは `serde_json` が作る（ここは位置と次元しか作らない）。

use serde::ser::{self, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// 出力を書き出して `{ "/a/0/b": "Power" }` を返す。
pub fn dims_of<T: Serialize>(value: &T) -> BTreeMap<String, String> {
    let mut r = Recorder::default();
    // 書き出しの失敗（あり得ないが）では、集めた分だけを返す。
    let _ = value.serialize(&mut r);
    r.dims
}

#[derive(Default)]
struct Recorder {
    path: Vec<String>,
    dims: BTreeMap<String, String>,
}

impl Recorder {
    fn pointer(&self) -> String {
        self.path
            .iter()
            .map(|s| format!("/{}", s.replace('~', "~0").replace('/', "~1")))
            .collect()
    }

    fn leaf(&mut self) {
        if let Some(dim) = sat_units::take_last_dimension() {
            let p = self.pointer();
            self.dims.insert(p, dim.to_owned());
        }
    }

    fn nested<T: Serialize + ?Sized>(&mut self, key: String, value: &T) -> Result<(), Error> {
        self.path.push(key);
        let r = value.serialize(&mut *self);
        self.path.pop();
        r
    }
}

/// 書き出しのエラー（使わない）。
#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

macro_rules! leaf_methods {
    ($($name:ident: $t:ty),+ $(,)?) => {
        $(fn $name(self, _v: $t) -> Result<(), Error> {
            self.leaf();
            Ok(())
        })+
    };
}

impl<'a> ser::Serializer for &'a mut Recorder {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Seq<'a>;
    type SerializeTuple = Seq<'a>;
    type SerializeTupleStruct = Seq<'a>;
    type SerializeTupleVariant = Seq<'a>;
    type SerializeMap = Map<'a>;
    type SerializeStruct = &'a mut Recorder;
    type SerializeStructVariant = &'a mut Recorder;

    leaf_methods!(
        serialize_bool: bool, serialize_i8: i8, serialize_i16: i16, serialize_i32: i32, serialize_i64: i64,
        serialize_u8: u8, serialize_u16: u16, serialize_u32: u32, serialize_u64: u64,
        serialize_f32: f32, serialize_f64: f64, serialize_char: char, serialize_str: &str, serialize_bytes: &[u8],
    );

    fn serialize_none(self) -> Result<(), Error> {
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Error> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Error> {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Error> {
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.nested(variant.to_owned(), value)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Seq<'a>, Error> {
        Ok(Seq {
            r: self,
            index: 0,
            pushed: false,
        })
    }
    fn serialize_tuple(self, _: usize) -> Result<Seq<'a>, Error> {
        Ok(Seq {
            r: self,
            index: 0,
            pushed: false,
        })
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Seq<'a>, Error> {
        Ok(Seq {
            r: self,
            index: 0,
            pushed: false,
        })
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Seq<'a>, Error> {
        self.path.push(variant.to_owned());
        Ok(Seq {
            r: self,
            index: 0,
            pushed: true,
        })
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Map<'a>, Error> {
        Ok(Map { r: self, key: None })
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<&'a mut Recorder, Error> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<&'a mut Recorder, Error> {
        // 外部タグの列挙 `{ "Variant": { … } }`。`end` で外す。
        self.path.push(variant.to_owned());
        Ok(self)
    }
}

/// 列・組の書き出し（添字を位置にする）。
pub struct Seq<'a> {
    r: &'a mut Recorder,
    index: usize,
    pushed: bool,
}

impl Seq<'_> {
    fn element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let i = self.index;
        self.index += 1;
        self.r.nested(i.to_string(), value)
    }

    fn finish(self) -> Result<(), Error> {
        if self.pushed {
            self.r.path.pop();
        }
        Ok(())
    }
}

impl ser::SerializeSeq for Seq<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.element(value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}

impl ser::SerializeTuple for Seq<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.element(value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}

impl ser::SerializeTupleStruct for Seq<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.element(value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}

impl ser::SerializeTupleVariant for Seq<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.element(value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}

/// 表の書き出し（キーの文字列を位置にする）。
pub struct Map<'a> {
    r: &'a mut Recorder,
    key: Option<String>,
}

impl ser::SerializeMap for Map<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        self.key = Some(
            serde_json::to_value(key)
                .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned))
                .map_err(|e| Error(e.to_string()))?,
        );
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self.key.take().unwrap_or_default();
        self.r.nested(key, value)
    }
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl ser::SerializeStruct for &mut Recorder {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.nested(key.to_owned(), value)
    }
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut Recorder {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.nested(key.to_owned(), value)
    }
    fn end(self) -> Result<(), Error> {
        self.path.pop();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::dims_of;
    use sat_units::typed::{Power, Qty, Time};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Inner {
        p: Qty<Power>,
        ratio: f64,
    }

    #[derive(Serialize)]
    struct Out {
        items: Vec<Inner>,
        t: Option<Qty<Time>>,
        map: std::collections::BTreeMap<String, Qty<Power>>,
    }

    #[test]
    fn records_pointer_and_dimension_of_every_qty() {
        let out = Out {
            items: vec![Inner {
                p: Qty::from_raw(1.0),
                ratio: 0.5,
            }],
            t: Some(Qty::from_raw(3.0)),
            map: [("a/b".to_owned(), Qty::from_raw(2.0))].into(),
        };
        let d = dims_of(&out);
        assert_eq!(d.get("/items/0/p").map(String::as_str), Some("Power"));
        assert_eq!(d.get("/t").map(String::as_str), Some("Time"));
        assert_eq!(d.get("/map/a~1b").map(String::as_str), Some("Power"));
        assert!(!d.contains_key("/items/0/ratio"));
    }
}
