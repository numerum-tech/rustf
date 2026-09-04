//! Deserializer for HTML form bodies.
//!
//! `ctx.body_form_typed::<T>()` used to bridge through `serde_json`, which
//! turned every field into a JSON string; a struct field typed `f64` or `bool`
//! then failed with a confusing "invalid type: string" error. This module
//! deserializes straight from the parsed [`FormValue`] map, parsing primitives
//! out of their string form the way an HTML form actually delivers them, and
//! mapping [`FormValue::Multiple`] onto sequence fields.

use std::collections::HashMap;
use std::fmt;

use serde::de::{
    self, DeserializeOwned, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};
use serde::forward_to_deserialize_any;

use crate::error::{Error as RustfError, Result};
use crate::http::request::FormValue;

/// Deserialize a parsed form body into `T`.
pub(crate) fn from_form_data<T: DeserializeOwned>(map: &HashMap<String, FormValue>) -> Result<T> {
    T::deserialize(FormDeserializer { map })
        .map_err(|error| RustfError::InvalidInput(error.to_string()))
}

/// Error type for form deserialization.
///
/// `serde` requires the deserializer's error type to implement
/// [`de::Error`], which `rustf::Error` does not; failures are converted to
/// [`RustfError::InvalidInput`] at the boundary in [`from_form_data`].
#[derive(Debug)]
pub(crate) struct FormError(String);

impl fmt::Display for FormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FormError {}

impl de::Error for FormError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        FormError(msg.to_string())
    }
}

type FormResult<T> = std::result::Result<T, FormError>;

/// Top-level deserializer over the whole form body.
struct FormDeserializer<'de> {
    map: &'de HashMap<String, FormValue>,
}

impl<'de> de::Deserializer<'de> for FormDeserializer<'de> {
    type Error = FormError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_map(FormMapAccess {
            entries: self.map.iter(),
            value: None,
        })
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> FormResult<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map struct enum
        identifier ignored_any
    }
}

/// Walks the form's key/value pairs for `deserialize_map`.
struct FormMapAccess<'de> {
    entries: std::collections::hash_map::Iter<'de, String, FormValue>,
    value: Option<(&'de str, &'de FormValue)>,
}

impl<'de> MapAccess<'de> for FormMapAccess<'de> {
    type Error = FormError;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> FormResult<Option<K::Value>> {
        match self.entries.next() {
            Some((key, value)) => {
                self.value = Some((key.as_str(), value));
                seed.deserialize(key.as_str().into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> FormResult<V::Value> {
        let (key, value) = self
            .value
            .take()
            .ok_or_else(|| de::Error::custom("form value requested before its key"))?;
        seed.deserialize(ValueDeserializer { key, value })
    }
}

/// Deserializer for one form field, which may hold one or many values.
struct ValueDeserializer<'de> {
    key: &'de str,
    value: &'de FormValue,
}

impl<'de> ValueDeserializer<'de> {
    /// The scalar view of this field: the only value, or the first of many.
    fn scalar(&self) -> FieldDeserializer<'de> {
        FieldDeserializer {
            key: self.key,
            value: self.value.as_string(),
        }
    }
}

/// Delegates a scalar `deserialize_*` call to the field's first value.
///
/// Forwarding these to `deserialize_any` instead would hand every visitor a
/// string, which is exactly the bug this module replaced.
macro_rules! delegate_to_scalar {
    ($($method:ident;)*) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
                self.scalar().$method(visitor)
            }
        )*
    };
}

impl<'de> de::Deserializer<'de> for ValueDeserializer<'de> {
    type Error = FormError;

    delegate_to_scalar! {
        deserialize_bool; deserialize_i8; deserialize_i16; deserialize_i32;
        deserialize_i64; deserialize_i128; deserialize_u8; deserialize_u16;
        deserialize_u32; deserialize_u64; deserialize_u128; deserialize_f32;
        deserialize_f64; deserialize_char; deserialize_str; deserialize_string;
        deserialize_bytes; deserialize_byte_buf; deserialize_unit;
        deserialize_identifier; deserialize_ignored_any;
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        // `Some` wraps this same deserializer rather than the scalar view, so
        // an `Option<Vec<T>>` field still sees every value.
        if matches!(self.value, FormValue::Single(s) if s.is_empty()) {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> FormResult<V::Value> {
        self.scalar().deserialize_unit_struct(name, visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        self.scalar().deserialize_map(visitor)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> FormResult<V::Value> {
        self.scalar().deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> FormResult<V::Value> {
        self.scalar().deserialize_enum(name, variants, visitor)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_seq(FormSeqAccess {
            key: self.key,
            values: self.value.as_array().into_iter(),
        })
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> FormResult<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> FormResult<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> FormResult<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        match self.value {
            FormValue::Single(_) => self.scalar().deserialize_any(visitor),
            FormValue::Multiple(_) => self.deserialize_seq(visitor),
        }
    }
}

/// Yields the individual values of a multi-value field.
struct FormSeqAccess<'de> {
    key: &'de str,
    values: std::vec::IntoIter<&'de str>,
}

impl<'de> SeqAccess<'de> for FormSeqAccess<'de> {
    type Error = FormError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> FormResult<Option<T::Value>> {
        match self.values.next() {
            Some(value) => seed
                .deserialize(FieldDeserializer {
                    key: self.key,
                    value,
                })
                .map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

/// Deserializer for a single string value, parsing primitives out of it.
struct FieldDeserializer<'de> {
    key: &'de str,
    value: &'de str,
}

impl<'de> FieldDeserializer<'de> {
    fn parse<T>(&self, type_name: &str) -> FormResult<T>
    where
        T: std::str::FromStr,
    {
        self.value.trim().parse::<T>().map_err(|_| {
            de::Error::custom(format!(
                "Field '{}' must be a valid {} (got '{}')",
                self.key, type_name, self.value
            ))
        })
    }
}

/// Generates the numeric `deserialize_*` methods, which all parse the field's
/// string form and name the expected type in the error.
macro_rules! deserialize_parsed {
    ($($method:ident => $ty:ty, $visit:ident, $name:literal;)*) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
                visitor.$visit(self.parse::<$ty>($name)?)
            }
        )*
    };
}

impl<'de> de::Deserializer<'de> for FieldDeserializer<'de> {
    type Error = FormError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_str(self.value)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_str(self.value)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_str(self.value)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        // An unchecked checkbox is simply absent, so a present-but-empty value
        // is treated as false rather than an error.
        match self.value.trim() {
            "true" | "1" | "yes" | "on" | "checked" => visitor.visit_bool(true),
            "false" | "0" | "no" | "off" | "" => visitor.visit_bool(false),
            other => Err(de::Error::custom(format!(
                "Field '{}' must be a valid boolean (got '{}')",
                self.key, other
            ))),
        }
    }

    deserialize_parsed! {
        deserialize_i8   => i8,   visit_i8,  "i8";
        deserialize_i16  => i16,  visit_i16, "i16";
        deserialize_i32  => i32,  visit_i32, "i32";
        deserialize_i64  => i64,  visit_i64, "i64";
        deserialize_i128 => i128, visit_i128, "i128";
        deserialize_u8   => u8,   visit_u8,  "u8";
        deserialize_u16  => u16,  visit_u16, "u16";
        deserialize_u32  => u32,  visit_u32, "u32";
        deserialize_u64  => u64,  visit_u64, "u64";
        deserialize_u128 => u128, visit_u128, "u128";
        deserialize_f32  => f32,  visit_f32, "f32";
        deserialize_f64  => f64,  visit_f64, "f64";
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        let mut chars = self.value.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(de::Error::custom(format!(
                "Field '{}' must be a single character (got '{}')",
                self.key, self.value
            ))),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_bytes(self.value.as_bytes())
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_byte_buf(self.value.as_bytes().to_vec())
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        // A form submits an untouched field as the empty string, which means
        // "not provided" for an `Option` field.
        if self.value.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> FormResult<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> FormResult<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        // A field that arrived once but is typed as a sequence is a one-element
        // sequence — the case of a single checked checkbox in a `tags[]` group.
        visitor.visit_seq(FormSeqAccess {
            key: self.key,
            values: vec![self.value].into_iter(),
        })
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> FormResult<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> FormResult<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> FormResult<V::Value> {
        Err(de::Error::custom(format!(
            "Field '{}' is a flat form value and cannot be deserialized into a map",
            self.key
        )))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> FormResult<V::Value> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> FormResult<V::Value> {
        visitor.visit_enum(UnitVariantAccess { value: self.value })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_str(self.value)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> FormResult<V::Value> {
        visitor.visit_unit()
    }
}

/// A form can only name a unit variant (`status=active`), never carry payload.
struct UnitVariantAccess<'de> {
    value: &'de str,
}

impl<'de> EnumAccess<'de> for UnitVariantAccess<'de> {
    type Error = FormError;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> FormResult<(V::Value, Self)> {
        let variant = seed.deserialize(self.value.into_deserializer())?;
        Ok((variant, self))
    }
}

impl<'de> VariantAccess<'de> for UnitVariantAccess<'de> {
    type Error = FormError;

    fn unit_variant(self) -> FormResult<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, _seed: T) -> FormResult<T::Value> {
        Err(de::Error::custom(
            "form values cannot populate a newtype enum variant",
        ))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> FormResult<V::Value> {
        Err(de::Error::custom(
            "form values cannot populate a tuple enum variant",
        ))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> FormResult<V::Value> {
        Err(de::Error::custom(
            "form values cannot populate a struct enum variant",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn form(pairs: &[(&str, FormValue)]) -> HashMap<String, FormValue> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn single(value: &str) -> FormValue {
        FormValue::Single(value.to_string())
    }

    fn multiple(values: &[&str]) -> FormValue {
        FormValue::Multiple(values.iter().map(|v| v.to_string()).collect())
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Product {
        name: String,
        price: f64,
        quantity: i64,
        in_stock: bool,
        description: Option<String>,
        tags: Vec<String>,
    }

    #[test]
    fn deserializes_mixed_primitive_types() {
        let data = form(&[
            ("name", single("Widget")),
            ("price", single("19.99")),
            ("quantity", single("42")),
            ("in_stock", single("on")),
            ("description", single("A widget")),
            ("tags", multiple(&["new", "sale"])),
        ]);

        let product: Product = from_form_data(&data).unwrap();
        assert_eq!(
            product,
            Product {
                name: "Widget".to_string(),
                price: 19.99,
                quantity: 42,
                in_stock: true,
                description: Some("A widget".to_string()),
                tags: vec!["new".to_string(), "sale".to_string()],
            }
        );
    }

    #[test]
    fn empty_optional_field_is_none() {
        let data = form(&[
            ("name", single("Widget")),
            ("price", single("1")),
            ("quantity", single("1")),
            ("in_stock", single("")),
            ("description", single("")),
            ("tags", single("solo")),
        ]);

        let product: Product = from_form_data(&data).unwrap();
        assert_eq!(product.description, None);
        // A present-but-empty checkbox reads as false, not an error.
        assert!(!product.in_stock);
        // A single value satisfies a `Vec` field.
        assert_eq!(product.tags, vec!["solo".to_string()]);
    }

    #[test]
    fn missing_optional_field_is_none() {
        #[derive(Debug, Deserialize)]
        struct Partial {
            name: String,
            nickname: Option<String>,
        }

        let data = form(&[("name", single("Widget"))]);
        let parsed: Partial = from_form_data(&data).unwrap();
        assert_eq!(parsed.name, "Widget");
        assert_eq!(parsed.nickname, None);
    }

    #[test]
    fn bad_number_names_the_field_and_type() {
        let data = form(&[
            ("name", single("Widget")),
            ("price", single("not-a-price")),
            ("quantity", single("1")),
            ("in_stock", single("true")),
            ("description", single("")),
            ("tags", single("x")),
        ]);

        let error = from_form_data::<Product>(&data).unwrap_err().to_string();
        assert!(
            error.contains("price") && error.contains("f64"),
            "error should name the field and the expected type, got: {}",
            error
        );
    }

    #[test]
    fn missing_required_field_is_reported() {
        let data = form(&[("price", single("1.0"))]);
        let error = from_form_data::<Product>(&data).unwrap_err().to_string();
        assert!(
            error.contains("name"),
            "error should name the missing field, got: {}",
            error
        );
    }

    #[test]
    fn numeric_strings_stay_strings_when_the_field_is_a_string() {
        #[derive(Debug, Deserialize)]
        struct Coded {
            zip: String,
        }

        let data = form(&[("zip", single("01234"))]);
        let parsed: Coded = from_form_data(&data).unwrap();
        // Leading zeroes survive because nothing coerces to a number first.
        assert_eq!(parsed.zip, "01234");
    }

    #[test]
    fn multi_value_field_read_as_scalar_takes_the_first() {
        #[derive(Debug, Deserialize)]
        struct First {
            tag: String,
        }

        let data = form(&[("tag", multiple(&["a", "b"]))]);
        let parsed: First = from_form_data(&data).unwrap();
        assert_eq!(parsed.tag, "a");
    }

    #[test]
    fn unit_enum_variant_from_string() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(rename_all = "lowercase")]
        enum Status {
            Active,
            Archived,
        }

        #[derive(Debug, Deserialize)]
        struct WithStatus {
            status: Status,
        }

        let data = form(&[("status", single("archived"))]);
        let parsed: WithStatus = from_form_data(&data).unwrap();
        assert_eq!(parsed.status, Status::Archived);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        #[derive(Debug, Deserialize)]
        struct Small {
            name: String,
        }

        let data = form(&[("name", single("Widget")), ("_csrf_token", single("abc"))]);
        let parsed: Small = from_form_data(&data).unwrap();
        assert_eq!(parsed.name, "Widget");
    }
}
