//! 解析の出力の量の次元と列挙の値: 出力を型付きのまま書き出し、`Qty` の葉ごとに JSON Pointer → 次元名を、
//! 列挙の値ごとに JSON Pointer → (型名, 綴り) を集める。
//! `Qty::serialize` が書き出す直前に記録する次元（`sat_units::take_last_dimension`）を、数値の葉の位置と結び付けるだけ。
//! 列挙は serde が書き出しに渡す型名で見分ける（表示名は `analysis::labels`）。
//! 出力の JSON そのものは `serde_json` が作る（ここは位置と次元・列挙しか作らない）。

use serde::ser::{self, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// 出力の書き出しで集めたもの。
#[derive(Debug, Default)]
pub struct OutputMeta {
    /// JSON Pointer → 次元名（`{ "/a/0/b": "Power" }`）
    pub dims: BTreeMap<String, String>,
    /// JSON Pointer → (型名, 値の綴り)。外部タグの単位の値（`"gravity_gradient"`）と、内部タグの列挙のタグ（`kind`）
    pub variants: BTreeMap<String, (&'static str, String)>,
}

/// 出力を書き出して、量の次元と列挙の値を集める。
pub fn meta_of<T: Serialize>(value: &T) -> OutputMeta {
    let mut r = Recorder::default();
    // 書き出しの失敗（あり得ないが）では、集めた分だけを返す。
    let _ = value.serialize(&mut r);
    r.meta
}

/// 出力を書き出して `{ "/a/0/b": "Power" }` を返す。
#[cfg(test)]
pub fn dims_of<T: Serialize>(value: &T) -> BTreeMap<String, String> {
    meta_of(value).dims
}

/// 内部タグの列挙のタグの欄（sat-analysis の列挙はどれも `#[serde(tag = "kind")]`）。
const TAG: &str = "kind";

/// JSON Pointer（RFC 6901）の 1 区間の書き方（`~` → `~0`、`/` → `~1`）。**service の pointer の区間はここだけで書く**
/// （次元・列挙の位置と、判定の位置 `analysis::checks`。Web は `analysis/output.ts` の `jsonPointer`）。
pub(super) fn pointer_token(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

#[derive(Default)]
struct Recorder {
    path: Vec<String>,
    /// 書き出し中の構造体の型名（内側ほど後ろ）。内部タグの列挙はこの名前で書き出される
    structs: Vec<&'static str>,
    meta: OutputMeta,
}

impl Recorder {
    fn variant(&mut self, ty: &'static str, value: &str) {
        let p = self.pointer();
        self.meta.variants.insert(p, (ty, value.to_owned()));
    }

    fn pointer(&self) -> String {
        self.path
            .iter()
            .map(|s| format!("/{}", pointer_token(s)))
            .collect()
    }

    fn leaf(&mut self) {
        if let Some(dim) = sat_units::take_last_dimension() {
            let p = self.pointer();
            self.meta.dims.insert(p, dim.to_owned());
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
    fn serialize_unit_variant(
        self,
        ty: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), Error> {
        self.variant(ty, variant);
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
    fn serialize_struct(self, ty: &'static str, _: usize) -> Result<&'a mut Recorder, Error> {
        // `end` で外す
        self.structs.push(ty);
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
        if key == TAG
            && let (Some(&ty), Ok(serde_json::Value::String(tag))) =
                (self.structs.last(), serde_json::to_value(value))
        {
            // 内部タグの列挙（`{ "kind": "closes_above", … }`）は型名とタグの値を記録する
            self.path.push(key.to_owned());
            self.variant(ty, &tag);
            self.path.pop();
        }
        self.nested(key.to_owned(), value)
    }
    fn end(self) -> Result<(), Error> {
        self.structs.pop();
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
mod tests;
