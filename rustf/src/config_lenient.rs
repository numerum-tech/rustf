//! Reading configuration values that arrived as strings.
//!
//! A `"(env) NAME"` reference in a section the framework does not know stays
//! a string (see `config_refs`), because nothing says whether `"0042"` is a
//! number or a password. The type requested by the reader does: `Lenient`
//! deserializes a `serde_json::Value` like serde_json does, except that a
//! string is parsed when the target asks for a number or a boolean. A `String`
//! target still receives the text unchanged.
//!
//! Limits: inside an enum's payload, or a `#[serde(flatten)]` field, serde
//! does not pass the target type down, so those keep strict typing.

use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;
use serde_json::{Error, Value};

/// Deserialize `value` into `T`, parsing strings into numbers or booleans
/// where `T` asks for one.
pub(crate) fn from_value<T: de::DeserializeOwned>(value: &Value) -> Result<T, Error> {
    T::deserialize(Lenient(value))
}

struct Lenient<'a>(&'a Value);

macro_rules! lenient_number {
    ($method:ident, $parse:ty, $visit:ident) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            if let Value::String(text) = self.0 {
                if let Ok(number) = text.trim().parse::<$parse>() {
                    return visitor.$visit(number);
                }
            }
            self.deserialize_any(visitor)
        }
    };
}

impl<'de> Deserializer<'de> for Lenient<'_> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    visitor.visit_u64(u)
                } else if let Some(i) = n.as_i64() {
                    visitor.visit_i64(i)
                } else {
                    visitor.visit_f64(n.as_f64().unwrap_or(f64::NAN))
                }
            }
            Value::String(s) => visitor.visit_str(s),
            Value::Array(items) => visitor.visit_seq(Seq(items.iter())),
            Value::Object(map) => visitor.visit_map(Map {
                entries: map.iter(),
                value: None,
            }),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if let Value::String(text) = self.0 {
            match text.trim().to_ascii_lowercase().as_str() {
                "true" => return visitor.visit_bool(true),
                "false" => return visitor.visit_bool(false),
                _ => {}
            }
        }
        self.deserialize_any(visitor)
    }

    lenient_number!(deserialize_i8, i64, visit_i64);
    lenient_number!(deserialize_i16, i64, visit_i64);
    lenient_number!(deserialize_i32, i64, visit_i64);
    lenient_number!(deserialize_i64, i64, visit_i64);
    lenient_number!(deserialize_i128, i128, visit_i128);
    lenient_number!(deserialize_u8, u64, visit_u64);
    lenient_number!(deserialize_u16, u64, visit_u64);
    lenient_number!(deserialize_u32, u64, visit_u64);
    lenient_number!(deserialize_u64, u64, visit_u64);
    lenient_number!(deserialize_u128, u128, visit_u128);
    lenient_number!(deserialize_f32, f64, visit_f64);
    lenient_number!(deserialize_f64, f64, visit_f64);

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        // serde_json's own enum handling; payloads are typed strictly.
        self.0.clone().deserialize_enum(name, variants, visitor)
    }

    forward_to_deserialize_any! {
        char str string bytes byte_buf unit unit_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct Seq<'a>(std::slice::Iter<'a, Value>);

impl<'de> SeqAccess<'de> for Seq<'_> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        self.0
            .next()
            .map(|item| seed.deserialize(Lenient(item)))
            .transpose()
    }
}

struct Map<'a> {
    entries: serde_json::map::Iter<'a>,
    value: Option<&'a Value>,
}

impl<'de> MapAccess<'de> for Map<'_> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match self.entries.next() {
            Some((key, value)) => {
                self.value = Some(value);
                // Through `Lenient` too, so `HashMap<u16, _>` keys still parse.
                seed.deserialize(Lenient(&Value::String(key.clone())))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value, Error> {
        let value = self
            .value
            .take()
            .ok_or_else(|| de::Error::custom("value requested before its key"))?;
        seed.deserialize(Lenient(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Mail {
        port: u16,
        tls: bool,
        password: String,
        ratio: f64,
        retries: Option<u32>,
        hosts: Vec<String>,
    }

    #[test]
    fn strings_become_what_the_target_asks_for() {
        let value = json!({
            "port": "2525",
            "tls": "TRUE",
            "password": "0042",
            "ratio": "0.5",
            "retries": "3",
            "hosts": ["123", "mail.example.com"],
        });

        let mail: Mail = from_value(&value).unwrap();

        assert_eq!(
            mail,
            Mail {
                port: 2525,
                tls: true,
                password: "0042".into(),
                ratio: 0.5,
                retries: Some(3),
                hosts: vec!["123".into(), "mail.example.com".into()],
            }
        );
    }

    #[test]
    fn real_types_still_work_and_bad_text_still_fails() {
        let port: u16 = from_value(&json!(8080)).unwrap();
        assert_eq!(port, 8080);
        assert!(from_value::<u16>(&json!("eighty")).is_err());
        assert!(from_value::<u16>(&json!("70000")).is_err());
        assert!(from_value::<u32>(&json!("-1")).is_err());
        // A number is not silently turned into a string.
        assert!(from_value::<String>(&json!(42)).is_err());
    }

    #[test]
    fn numeric_map_keys_still_parse() {
        let limits: std::collections::HashMap<u16, u32> =
            from_value(&json!({ "1": 10, "2": "20" })).unwrap();
        assert_eq!(limits[&1], 10);
        assert_eq!(limits[&2], 20);
    }
}
