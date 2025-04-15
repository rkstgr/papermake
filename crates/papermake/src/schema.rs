//! Schema definition for templates

use serde::{Serialize, Deserialize};

use crate::error::{PapermakeError, Result};

/// Supported field types in a schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Date,
    Object(Box<Schema>),
    Array(Box<FieldType>),
}

/// A field in a schema with metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaField {
    pub key: String,
    pub label: Option<String>,
    pub field_type: FieldType,
    pub required: bool,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// A schema defining the structure of data for a template
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Schema {
    pub fields: Vec<SchemaField>,
}

impl Schema {
    /// Create a new empty schema
    pub fn new() -> Self {
        Schema { fields: Vec::new() }
    }
    
    /// Add a field to the schema
    pub fn add_field(&mut self, field: SchemaField) -> &mut Self {
        self.fields.push(field);
        self
    }
    
    /// Validate that provided data matches this schema
    pub fn validate(&self, data: &serde_json::Value) -> Result<()> {
        if !data.is_object() {
            return Err(PapermakeError::SchemaValidation(
                "Root data must be an object".to_string()
            ));
        }
        
        let data_obj = data.as_object().unwrap();
        
        for field in &self.fields {
            if field.required && !data_obj.contains_key(&field.key) {
                return Err(PapermakeError::SchemaValidation(
                    format!("Required field '{}' is missing", field.key)
                ));
            }
            
            if let Some(value) = data_obj.get(&field.key) {
                self.validate_field_type(&field.field_type, value, &field.key)?;
            }
        }
        
        Ok(())
    }
    
    // Validate that a value matches the expected type
    fn validate_field_type(&self, field_type: &FieldType, value: &serde_json::Value, path: &str) -> Result<()> {
        match field_type {
            FieldType::String => {
                if !value.is_string() {
                    return Err(PapermakeError::SchemaValidation(
                        format!("Field '{}' must be a string", path)
                    ));
                }
            },
            FieldType::Number => {
                if !value.is_number() {
                    return Err(PapermakeError::SchemaValidation(
                        format!("Field '{}' must be a number", path)
                    ));
                }
            },
            FieldType::Boolean => {
                if !value.is_boolean() {
                    return Err(PapermakeError::SchemaValidation(
                        format!("Field '{}' must be a boolean", path)
                    ));
                }
            },
            FieldType::Date => {
                // Simple validation - just check if it's a string for now
                // In a real implementation, you'd parse and validate the date format
                if !value.is_string() {
                    return Err(PapermakeError::SchemaValidation(
                        format!("Field '{}' must be a date string", path)
                    ));
                }
            },
            FieldType::Object(sub_schema) => {
                if !value.is_object() {
                    return Err(PapermakeError::SchemaValidation(
                        format!("Field '{}' must be an object", path)
                    ));
                }
                
                sub_schema.validate(value)?;
            },
            FieldType::Array(item_type) => {
                if !value.is_array() {
                    return Err(PapermakeError::SchemaValidation(
                        format!("Field '{}' must be an array", path)
                    ));
                }
                
                let array = value.as_array().unwrap();
                for (i, item) in array.iter().enumerate() {
                    self.validate_field_type(item_type, item, &format!("{}[{}]", path, i))?;
                }
            },
        }
        
        Ok(())
    }
}