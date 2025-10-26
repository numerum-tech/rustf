//! Object manipulation utilities for RustF framework
//!
//! This module provides utilities for working with JSON objects and nested data structures,
//! including safe property access, modification, and merging operations.

use crate::error::{Error, Result};
use serde_json::{Map, Value};

/// Get a nested property from a JSON object safely
///
/// Traverses a JSON object using a dot-separated path to access nested properties.
/// Returns None if any part of the path doesn't exist or is not accessible.
///
/// # Arguments
/// * `obj` - JSON object to search in
/// * `path` - Dot-separated path (e.g., "user.profile.name")
///
/// # Example
/// ```rust,ignore
/// use serde_json::json;
///
/// let data = json!({
///     "user": {
///         "profile": {
///             "name": "John Doe",
///             "age": 30
///         },
///         "settings": {
///             "theme": "dark"
///         }
///     }
/// });
///
/// let name = get(&data, "user.profile.name");
/// assert_eq!(name, Some(&json!("John Doe")));
///
/// let missing = get(&data, "user.profile.email");
/// assert_eq!(missing, None);
/// ```
pub fn get<'a>(obj: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = obj;

    for part in parts {
        match current {
            Value::Object(map) => {
                current = map.get(part)?;
            }
            Value::Array(arr) => {
                if let Ok(index) = part.parse::<usize>() {
                    current = arr.get(index)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Set a nested property in a JSON object
///
/// Creates nested objects as needed to set a value at the specified path.
/// If intermediate objects don't exist, they will be created as empty objects.
///
/// # Arguments
/// * `obj` - Mutable JSON object to modify
/// * `path` - Dot-separated path (e.g., "user.profile.name")
/// * `value` - Value to set
///
/// # Example
/// ```rust,ignore
/// use serde_json::json;
///
/// let mut data = json!({});
/// set(&mut data, "user.profile.name", json!("Jane Doe")).unwrap();
///
/// assert_eq!(data["user"]["profile"]["name"], "Jane Doe");
/// ```
pub fn set(obj: &mut Value, path: &str, value: Value) -> Result<()> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err(Error::template("Empty path provided".to_string()));
    }

    // Ensure root is an object
    if !obj.is_object() {
        *obj = Value::Object(Map::new());
    }

    let mut current = obj;

    // Navigate to the parent of the target property
    for part in &parts[..parts.len() - 1] {
        if let Value::Object(map) = current {
            let entry = map
                .entry(*part)
                .or_insert_with(|| Value::Object(Map::new()));
            if !entry.is_object() {
                *entry = Value::Object(Map::new());
            }
            current = entry;
        } else {
            return Err(Error::template(format!(
                "Cannot set property '{}' on non-object",
                part
            )));
        }
    }

    // Set the final property
    if let Value::Object(map) = current {
        let final_key = parts[parts.len() - 1];
        map.insert(final_key.to_string(), value);
        Ok(())
    } else {
        Err(Error::template(
            "Cannot set property on non-object".to_string(),
        ))
    }
}

/// Remove a nested property from a JSON object
///
/// # Arguments
/// * `obj` - Mutable JSON object to modify
/// * `path` - Dot-separated path to remove
///
/// # Example
/// ```rust,ignore
/// let mut data = json!({"user": {"name": "John", "age": 30}});
/// remove_nested_property(&mut data, "user.age").unwrap();
/// // data now contains {"user": {"name": "John"}}
/// ```
pub fn remove_nested_property(obj: &mut Value, path: &str) -> Result<Option<Value>> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err(Error::template("Empty path provided".to_string()));
    }

    if parts.len() == 1 {
        // Remove from root object
        if let Value::Object(map) = obj {
            return Ok(map.remove(parts[0]));
        } else {
            return Err(Error::template(
                "Cannot remove property from non-object".to_string(),
            ));
        }
    }

    // Navigate to parent and remove from there
    let parent_path = parts[..parts.len() - 1].join(".");
    let final_key = parts[parts.len() - 1];

    if let Some(Value::Object(parent_map)) = get_mut(obj, &parent_path) {
        Ok(parent_map.remove(final_key))
    } else {
        Ok(None)
    }
}

/// Get mutable reference to nested property
///
/// Helper function to get a mutable reference to a nested property.
///
/// # Arguments
/// * `obj` - Mutable JSON object
/// * `path` - Dot-separated path
fn get_mut<'a>(obj: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = obj;

    for part in parts {
        match current {
            Value::Object(map) => {
                current = map.get_mut(part)?;
            }
            Value::Array(arr) => {
                if let Ok(index) = part.parse::<usize>() {
                    current = arr.get_mut(index)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Deep merge two JSON objects
///
/// Recursively merges properties from source into target.
/// Properties in source will overwrite properties in target.
///
/// # Arguments
/// * `target` - Target object to merge into
/// * `source` - Source object to merge from
///
/// # Example
/// ```rust,ignore
/// let mut target = json!({"a": 1, "b": {"c": 2}});
/// let source = json!({"b": {"d": 3}, "e": 4});
///
/// deep_merge(&mut target, &source);
/// // target now contains {"a": 1, "b": {"c": 2, "d": 3}, "e": 4}
/// ```
pub fn deep_merge(target: &mut Value, source: &Value) {
    match (target.as_object_mut(), source.as_object()) {
        (Some(target_map), Some(source_map)) => {
            for (key, source_value) in source_map {
                match target_map.get_mut(key) {
                    Some(target_value) => {
                        deep_merge(target_value, source_value);
                    }
                    None => {
                        target_map.insert(key.clone(), source_value.clone());
                    }
                }
            }
        }
        _ => {
            *target = source.clone();
        }
    }
}

/// Shallow merge two JSON objects
///
/// Merges only top-level properties from source into target.
///
/// # Arguments
/// * `target` - Target object to merge into
/// * `source` - Source object to merge from
///
/// # Example
/// ```rust,ignore
/// let mut target = json!({"a": 1, "b": {"c": 2}});
/// let source = json!({"b": {"d": 3}, "e": 4});
///
/// shallow_merge(&mut target, &source);
/// // target now contains {"a": 1, "b": {"d": 3}, "e": 4}
/// ```
pub fn shallow_merge(target: &mut Value, source: &Value) {
    if let (Value::Object(target_map), Value::Object(source_map)) = (target, source) {
        for (key, value) in source_map {
            target_map.insert(key.clone(), value.clone());
        }
    }
}

/// Deep clone a JSON value
///
/// Creates a deep copy of a JSON value, ensuring no shared references.
///
/// # Arguments
/// * `value` - Value to clone
///
/// # Example
/// ```rust,ignore
/// let original = json!({"user": {"name": "John", "settings": {"theme": "dark"}}});
/// let cloned = deep_clone(&original);
///
/// // cloned is completely independent of original
/// ```
pub fn deep_clone(value: &Value) -> Value {
    value.clone()
}

/// Flatten nested JSON object to dot-notation keys
///
/// Converts nested object structure to flat key-value pairs using dot notation.
///
/// # Arguments
/// * `obj` - JSON object to flatten
/// * `prefix` - Prefix for keys (usually empty string for root)
///
/// # Example
/// ```rust,ignore
/// let nested = json!({"user": {"profile": {"name": "John", "age": 30}}});
/// let flattened = flatten_object(&nested, "");
///
/// // Returns: {"user.profile.name": "John", "user.profile.age": 30}
/// ```
pub fn flatten_object(obj: &Value, prefix: &str) -> Map<String, Value> {
    let mut result = Map::new();

    match obj {
        Value::Object(map) => {
            for (key, value) in map {
                let new_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };

                match value {
                    Value::Object(_) => {
                        let nested = flatten_object(value, &new_key);
                        result.extend(nested);
                    }
                    _ => {
                        result.insert(new_key, value.clone());
                    }
                }
            }
        }
        _ => {
            if !prefix.is_empty() {
                result.insert(prefix.to_string(), obj.clone());
            }
        }
    }

    result
}

/// Check if a JSON object has a nested property
///
/// # Arguments
/// * `obj` - JSON object to check
/// * `path` - Dot-separated path to check
///
/// # Example
/// ```rust,ignore
/// let data = json!({"user": {"name": "John"}});
/// assert!(has_nested_property(&data, "user.name"));
/// assert!(!has_nested_property(&data, "user.email"));
/// ```
pub fn has_nested_property(obj: &Value, path: &str) -> bool {
    get(obj, path).is_some()
}

/// Get all keys from a JSON object (including nested keys with dot notation)
///
/// # Arguments
/// * `obj` - JSON object to get keys from
///
/// # Example
/// ```rust,ignore
/// let data = json!({"user": {"name": "John", "age": 30}, "active": true});
/// let keys = get_all_keys(&data);
/// // Returns: ["user.name", "user.age", "active"]
/// ```
pub fn get_all_keys(obj: &Value) -> Vec<String> {
    let flattened = flatten_object(obj, "");
    flattened.keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get() {
        let data = json!({
            "user": {
                "profile": {
                    "name": "John Doe",
                    "age": 30
                },
                "settings": {
                    "theme": "dark"
                }
            },
            "items": [1, 2, 3]
        });

        // Test nested object access
        assert_eq!(get(&data, "user.profile.name"), Some(&json!("John Doe")));
        assert_eq!(get(&data, "user.settings.theme"), Some(&json!("dark")));

        // Test array access
        assert_eq!(get(&data, "items.0"), Some(&json!(1)));
        assert_eq!(get(&data, "items.2"), Some(&json!(3)));

        // Test missing properties
        assert_eq!(get(&data, "user.profile.email"), None);
        assert_eq!(get(&data, "missing"), None);
        assert_eq!(get(&data, "items.10"), None);
    }

    #[test]
    fn test_set() {
        let mut data = json!({});

        // Set nested property in empty object
        set(&mut data, "user.profile.name", json!("Jane Doe")).unwrap();
        assert_eq!(data["user"]["profile"]["name"], "Jane Doe");

        // Set another property
        set(&mut data, "user.profile.age", json!(25)).unwrap();
        assert_eq!(data["user"]["profile"]["age"], 25);

        // Set property in different branch
        set(&mut data, "user.settings.theme", json!("light")).unwrap();
        assert_eq!(data["user"]["settings"]["theme"], "light");

        // Verify structure is correct
        assert_eq!(data["user"]["profile"]["name"], "Jane Doe");
        assert_eq!(data["user"]["profile"]["age"], 25);
    }

    #[test]
    fn test_remove_nested_property() {
        let mut data = json!({
            "user": {
                "name": "John",
                "age": 30,
                "email": "john@example.com"
            }
        });

        let removed = remove_nested_property(&mut data, "user.age").unwrap();
        assert_eq!(removed, Some(json!(30)));
        assert!(!has_nested_property(&data, "user.age"));
        assert_eq!(data["user"]["name"], "John");

        // Try to remove non-existent property
        let missing = remove_nested_property(&mut data, "user.phone").unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn test_deep_merge() {
        let mut target = json!({
            "a": 1,
            "b": {
                "c": 2,
                "d": 3
            }
        });

        let source = json!({
            "b": {
                "d": 4,
                "e": 5
            },
            "f": 6
        });

        deep_merge(&mut target, &source);

        assert_eq!(target["a"], 1);
        assert_eq!(target["b"]["c"], 2);
        assert_eq!(target["b"]["d"], 4); // Overwritten
        assert_eq!(target["b"]["e"], 5); // Added
        assert_eq!(target["f"], 6); // Added
    }

    #[test]
    fn test_shallow_merge() {
        let mut target = json!({
            "a": 1,
            "b": {"c": 2}
        });

        let source = json!({
            "b": {"d": 3},
            "e": 4
        });

        shallow_merge(&mut target, &source);

        assert_eq!(target["a"], 1);
        assert_eq!(target["b"]["d"], 3);
        assert!(!has_nested_property(&target, "b.c")); // Replaced, not merged
        assert_eq!(target["e"], 4);
    }

    #[test]
    fn test_flatten_object() {
        let nested = json!({
            "user": {
                "profile": {
                    "name": "John",
                    "age": 30
                },
                "active": true
            },
            "count": 5
        });

        let flattened = flatten_object(&nested, "");

        assert_eq!(flattened["user.profile.name"], json!("John"));
        assert_eq!(flattened["user.profile.age"], json!(30));
        assert_eq!(flattened["user.active"], json!(true));
        assert_eq!(flattened["count"], json!(5));
    }

    #[test]
    fn test_has_nested_property() {
        let data = json!({"user": {"name": "John", "age": 30}});

        assert!(has_nested_property(&data, "user"));
        assert!(has_nested_property(&data, "user.name"));
        assert!(has_nested_property(&data, "user.age"));
        assert!(!has_nested_property(&data, "user.email"));
        assert!(!has_nested_property(&data, "missing"));
    }

    #[test]
    fn test_get_all_keys() {
        let data = json!({
            "user": {
                "name": "John",
                "age": 30
            },
            "active": true
        });

        let keys = get_all_keys(&data);
        assert!(keys.contains(&"user.name".to_string()));
        assert!(keys.contains(&"user.age".to_string()));
        assert!(keys.contains(&"active".to_string()));
        assert_eq!(keys.len(), 3);
    }
}
