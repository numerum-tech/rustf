//! Schema type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Table definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Table name (for code generation, auto-assigned from YAML key if not specified)
    #[serde(default)]
    pub name: String,
    
    /// Database table name
    pub table: String,
    
    /// Database type (mysql, postgres, sqlite)
    pub database_type: Option<String>,
    
    /// Database name (actual database instance name)
    pub database_name: Option<String>,
    
    /// Element type (table, view, materialized_view)
    pub element_type: Option<String>,
    
    /// Schema version for migrations
    pub version: u32,
    
    /// Human-readable description
    pub description: Option<String>,
    
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// Extended AI guidance
    pub ai_context: Option<String>,
    
    /// Table fields
    #[serde(default)]
    pub fields: HashMap<String, Field>,
    
    /// Table relations
    #[serde(default)]
    pub relations: Relations,
    
    /// Table indexes
    #[serde(default)]
    pub indexes: Vec<Index>,
    
    /// Table constraints
    #[serde(default)]
    pub constraints: Vec<Constraint>,
}

/// Field definition
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    /// Field name (auto-assigned from YAML key if not specified)
    #[serde(default)]
    pub name: String,
    
    /// Database type (int, string(255), decimal(10,2), etc.)
    #[serde(rename = "type")]
    pub field_type: FieldType,
    
    /// Optional language-specific type (i32, String, Decimal)
    pub lang_type: Option<String>,
    
    /// PostgreSQL-specific enum type name (e.g., "user_role" for PostgreSQL enums)
    /// Used for proper type casting in PostgreSQL queries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres_type_name: Option<String>,
    
    /// Field constraints
    #[serde(flatten)]
    pub constraints: FieldConstraints,
    
    /// AI hint for code generation
    pub ai: Option<String>,
    
    /// Example value
    pub example: Option<serde_json::Value>,
}

/// Field type definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldType {
    /// Simple type (int, text, timestamp, etc.)
    Simple(String),
    
    /// Parameterized type (string(255), decimal(10,2))
    Parameterized {
        #[serde(rename = "type")]
        base_type: String,
        params: Vec<TypeParam>,
    },
    
    /// Enum type
    Enum {
        #[serde(rename = "type")]
        type_name: String,
        values: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        transitions: Option<HashMap<String, Vec<String>>>,
    },
    
    /// JSON type with schema
    Json {
        #[serde(rename = "type")]
        type_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: Option<serde_json::Value>,
    },
}

/// Type parameter for parameterized types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypeParam {
    Number(u32),
    String(String),
}

/// Field constraints
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldConstraints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto: Option<AutoGenerate>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<Validation>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_key: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<ForeignKeyAction>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<ForeignKeyAction>,
}

/// Auto-generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AutoGenerate {
    Boolean(bool),
    Type(String), // "create", "update"
}

/// Validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Validation {
    Simple(String), // "email"
    Complex(HashMap<String, serde_json::Value>),
}

/// Foreign key actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForeignKeyAction {
    Cascade,
    Restrict,
    SetNull,
    SetDefault,
    NoAction,
}

/// Table relations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Relations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_many: Option<HashMap<String, HasMany>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_one: Option<HashMap<String, HasOne>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub belongs_to: Option<HashMap<String, BelongsTo>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub many_to_many: Option<HashMap<String, ManyToMany>>,
}

/// Has many relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasMany {
    pub model: String,
    pub local_field: String,
    pub foreign_field: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cascade: Option<ForeignKeyAction>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<String>,
}

/// Has one relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasOne {
    pub model: String,
    pub local_field: String,
    pub foreign_field: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cascade: Option<ForeignKeyAction>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<String>,
}

/// Belongs to relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BelongsTo {
    pub model: String,
    pub local_field: String,
    pub foreign_field: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<ForeignKeyAction>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<ForeignKeyAction>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<String>,
}

/// Many to many relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManyToMany {
    pub model: String,
    pub through: String,
    pub local_through_field: String,
    pub foreign_through_field: String,
    pub local_field: String,
    pub foreign_field: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<String>,
}

/// Index definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Index {
    /// Simple field index
    Simple(String),
    
    /// Composite index
    Composite(Vec<String>),
    
    /// Detailed index
    Detailed {
        fields: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unique: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "type")]
        index_type: Option<String>,
    },
}

/// Table constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    
    pub message: String,
}

impl FieldType {
    /// Parse a type string like "string(255)" or "decimal(10,2)"
    pub fn parse(type_str: &str) -> Self {
        // Check for parameterized types
        if let Some(paren_pos) = type_str.find('(') {
            let base_type = type_str[..paren_pos].to_string();
            let params_str = &type_str[paren_pos + 1..type_str.len() - 1];
            
            let params: Vec<TypeParam> = params_str
                .split(',')
                .map(|p| {
                    let p = p.trim();
                    if let Ok(num) = p.parse::<u32>() {
                        TypeParam::Number(num)
                    } else {
                        TypeParam::String(p.to_string())
                    }
                })
                .collect();
                
            FieldType::Parameterized { base_type, params }
        } else {
            FieldType::Simple(type_str.to_string())
        }
    }
    
    /// Get the base type name
    pub fn base_type(&self) -> &str {
        match self {
            FieldType::Simple(t) => t,
            FieldType::Parameterized { base_type, .. } => base_type,
            FieldType::Enum { .. } => "enum",
            FieldType::Json { .. } => "json",
        }
    }
}

/// Custom deserializer for FieldType that handles string parsing
#[allow(dead_code)]
fn deserialize_field_type<'de, D>(deserializer: D) -> Result<FieldType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    
    let value = serde_json::Value::deserialize(deserializer)?;
    
    match value {
        serde_json::Value::String(type_str) => {
            Ok(FieldType::parse(&type_str))
        },
        serde_json::Value::Object(obj) => {
            // Handle enum and json types
            if let Some(type_name) = obj.get("type").and_then(|v| v.as_str()) {
                match type_name {
                    "enum" => {
                        let values = obj.get("values")
                            .and_then(|v| v.as_array())
                            .ok_or_else(|| D::Error::custom("enum type requires 'values' field"))?
                            .iter()
                            .map(|v| v.as_str().unwrap_or("").to_string())
                            .collect();
                        
                        let transitions = obj.get("transitions")
                            .and_then(|v| serde_json::from_value(v.clone()).ok());
                            
                        Ok(FieldType::Enum {
                            type_name: "enum".to_string(),
                            values,
                            transitions,
                        })
                    },
                    "json" | "jsonb" => {
                        let schema = obj.get("schema").cloned();
                        Ok(FieldType::Json {
                            type_name: type_name.to_string(),
                            schema,
                        })
                    },
                    _ => Ok(FieldType::Simple(type_name.to_string()))
                }
            } else {
                Err(D::Error::custom("Field type object must have 'type' field"))
            }
        },
        _ => Err(D::Error::custom("Field type must be string or object"))
    }
}

/// Custom deserializer for Field that handles enum types with values at the same level
impl<'de> Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum FieldKey {
            Name,
            Type,
            Values,
            Transitions,
            LangType,
            PostgresTypeName,
            Ai,
            Example,
            ColumnComment,
            // Constraint fields
            PrimaryKey,
            Auto,
            Unique,
            Required,
            Nullable,
            Default,
            Hidden,
            ForeignKey,
            MinLength,
            MaxLength,
            Min,
            Max,
            Pattern,
            Enum,
            Index,
            Indexed,
            SearchWeight,
            Validation,
            Computed,
            OnDelete,
            OnUpdate,
        }

        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a field definition")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Field, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                let mut field_type_str = None;
                let mut values = None;
                let mut transitions = None;
                let mut lang_type = None;
                let mut postgres_type_name = None;
                let mut ai = None;
                let mut example = None;
                let mut constraints = FieldConstraints::default();

                while let Some(key) = map.next_key::<FieldKey>()? {
                    match key {
                        FieldKey::Name => name = Some(map.next_value()?),
                        FieldKey::Type => field_type_str = Some(map.next_value::<String>()?),
                        FieldKey::Values => values = Some(map.next_value()?),
                        FieldKey::Transitions => transitions = Some(map.next_value()?),
                        FieldKey::LangType => lang_type = Some(map.next_value()?),
                        FieldKey::PostgresTypeName => postgres_type_name = Some(map.next_value()?),
                        FieldKey::Ai => ai = Some(map.next_value()?),
                        FieldKey::Example => example = Some(map.next_value()?),
                        // Constraint fields
                        FieldKey::PrimaryKey => constraints.primary_key = Some(map.next_value()?),
                        FieldKey::Auto => constraints.auto = Some(map.next_value()?),
                        FieldKey::Unique => constraints.unique = Some(map.next_value()?),
                        FieldKey::Required => constraints.required = Some(map.next_value()?),
                        FieldKey::Nullable => constraints.nullable = Some(map.next_value()?),
                        FieldKey::Default => constraints.default = Some(map.next_value()?),
                        FieldKey::Hidden => constraints.hidden = Some(map.next_value()?),
                        FieldKey::ForeignKey => constraints.foreign_key = Some(map.next_value()?),
                        FieldKey::MinLength => constraints.min_length = Some(map.next_value()?),
                        FieldKey::MaxLength => constraints.max_length = Some(map.next_value()?),
                        FieldKey::Min => constraints.min = Some(map.next_value()?),
                        FieldKey::Max => constraints.max = Some(map.next_value()?),
                        FieldKey::Pattern => constraints.pattern = Some(map.next_value()?),
                        // These fields are not used in constraints, skip them
                        FieldKey::Enum => { let _: Option<Vec<String>> = map.next_value()?; },
                        FieldKey::Index => { let _: Option<bool> = map.next_value()?; },
                        FieldKey::Indexed => { let _: Option<bool> = map.next_value()?; },
                        FieldKey::SearchWeight => { let _: Option<f64> = map.next_value()?; },
                        FieldKey::Validation => constraints.validate = Some(map.next_value()?),
                        FieldKey::ColumnComment => { let _: Option<String> = map.next_value()?; },
                        FieldKey::Computed => constraints.computed = Some(map.next_value()?),
                        FieldKey::OnDelete => constraints.on_delete = Some(map.next_value()?),
                        FieldKey::OnUpdate => constraints.on_update = Some(map.next_value()?),
                    }
                }

                let field_type = if let Some(type_str) = field_type_str {
                    match type_str.as_str() {
                        "enum" => {
                            // Handle enum type with values
                            if let Some(values) = values {
                                FieldType::Enum {
                                    type_name: "enum".to_string(),
                                    values,
                                    transitions,
                                }
                            } else {
                                // Fallback to simple enum type if no values provided
                                FieldType::Simple("enum".to_string())
                            }
                        }
                        "json" | "jsonb" => {
                            // Handle JSON types (could extend with schema if needed)
                            FieldType::Simple(type_str)
                        }
                        _ => {
                            // Parse as simple or parameterized type
                            FieldType::parse(&type_str)
                        }
                    }
                } else {
                    return Err(de::Error::missing_field("type"));
                };

                Ok(Field {
                    name: name.unwrap_or_default(),
                    field_type,
                    lang_type,
                    postgres_type_name,
                    constraints,
                    ai,
                    example,
                })
            }
        }

        deserializer.deserialize_map(FieldVisitor)
    }
}

impl Default for Table {
    fn default() -> Self {
        Self {
            name: String::new(),
            table: String::new(),
            database_type: None,
            database_name: None,
            element_type: None,
            version: 1,
            description: None,
            tags: Vec::new(),
            ai_context: None,
            fields: HashMap::new(),
            relations: Relations::default(),
            indexes: Vec::new(),
            constraints: Vec::new(),
        }
    }
}
