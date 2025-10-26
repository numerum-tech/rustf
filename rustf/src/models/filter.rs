use crate::database::SqlValue;
use crate::models::query_builder::{WhereCondition, WhereConnector};
use std::fmt::Debug;

/// A reusable filter that can be applied to any ModelQuery
///
/// Example usage:
/// ```rust
/// // Create a reusable filter
/// let active_users = ModelFilter::new()
///     .where_eq("is_active", true)
///     .where_not_null("verified_at");
///
/// // Apply to different queries
/// let count = Users::query()?
///     .apply_filter(&active_users)
///     .count()
///     .await?;
///
/// let users = Users::query()?
///     .apply_filter(&active_users)
///     .order_by("created_at", OrderDirection::Desc)
///     .limit(10)
///     .get()
///     .await?;
/// ```
#[derive(Clone, Debug, Default)]
pub struct ModelFilter {
    conditions: Vec<WhereCondition>,
}

impl ModelFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    /// Add a WHERE = condition
    pub fn where_eq<V: Into<SqlValue>>(mut self, field: &str, value: V) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "=".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE != condition
    pub fn where_ne<V: Into<SqlValue>>(mut self, field: &str, value: V) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "!=".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE > condition
    pub fn where_gt<V: Into<SqlValue>>(mut self, field: &str, value: V) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: ">".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE >= condition
    pub fn where_gte<V: Into<SqlValue>>(mut self, field: &str, value: V) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: ">=".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE < condition
    pub fn where_lt<V: Into<SqlValue>>(mut self, field: &str, value: V) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "<".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE <= condition
    pub fn where_lte<V: Into<SqlValue>>(mut self, field: &str, value: V) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "<=".to_string(),
            value: value.into(),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE LIKE condition
    pub fn where_like(mut self, field: &str, pattern: &str) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "LIKE".to_string(),
            value: SqlValue::String(pattern.to_string()),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE NOT LIKE condition
    pub fn where_not_like(mut self, field: &str, pattern: &str) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "NOT LIKE".to_string(),
            value: SqlValue::String(pattern.to_string()),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE IN condition
    pub fn where_in<V: Into<SqlValue>>(mut self, field: &str, values: Vec<V>) -> Self {
        // For IN conditions, we'll use a special format
        let values_vec: Vec<SqlValue> = values.into_iter().map(|v| v.into()).collect();
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "IN".to_string(),
            value: SqlValue::Array(values_vec),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE NOT IN condition
    pub fn where_not_in<V: Into<SqlValue>>(mut self, field: &str, values: Vec<V>) -> Self {
        let values_vec: Vec<SqlValue> = values.into_iter().map(|v| v.into()).collect();
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "NOT IN".to_string(),
            value: SqlValue::Array(values_vec),
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE IS NULL condition
    pub fn where_null(mut self, field: &str) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "IS NULL".to_string(),
            value: SqlValue::Null,
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE IS NOT NULL condition
    pub fn where_not_null(mut self, field: &str) -> Self {
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "IS NOT NULL".to_string(),
            value: SqlValue::Null,
            connector: WhereConnector::And,
        });
        self
    }

    /// Add a WHERE BETWEEN condition
    pub fn where_between<V: Into<SqlValue>>(mut self, field: &str, start: V, end: V) -> Self {
        // For BETWEEN, we'll use the first value and store the range in an array
        self.conditions.push(WhereCondition {
            column: field.to_string(),
            operator: "BETWEEN".to_string(),
            value: SqlValue::Array(vec![start.into(), end.into()]),
            connector: WhereConnector::And,
        });
        self
    }

    /// Combine with another filter using AND logic
    pub fn and(mut self, other: ModelFilter) -> Self {
        self.conditions.extend(other.conditions);
        self
    }

    /// Get the conditions for applying to a query
    pub(crate) fn get_conditions(&self) -> &[WhereCondition] {
        &self.conditions
    }

    /// Check if the filter has any conditions
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }

    /// Get the number of conditions
    pub fn len(&self) -> usize {
        self.conditions.len()
    }
}
