//! Macros for ergonomic schema and template definition

/// Macro for declarative schema definition
/// 
/// # Examples
/// 
/// ```rust
/// use papermake::{schema, FieldType};
/// 
/// let schema = schema! {
///     name: String,
///     email: String,
///     age?: Number,
///     active: Boolean
/// };
/// ```
#[macro_export]
macro_rules! schema {
    // Handle empty schema
    {} => {
        $crate::Schema::new()
    };
    
    // Required field: name: Type
    ($name:ident: $type:ident $(, $($rest:tt)*)?) => {
        {
            let mut builder = $crate::Schema::builder();
            builder = builder.field(stringify!($name), schema!(@type $type));
            $(
                builder = schema!(@parse builder, $($rest)*);
            )?
            builder.build()
        }
    };
    
    // Optional field: name?: Type
    ($name:ident ?: $type:ident $(, $($rest:tt)*)?) => {
        {
            let mut builder = $crate::Schema::builder();
            builder = builder.optional(stringify!($name), schema!(@type $type));
            $(
                builder = schema!(@parse builder, $($rest)*);
            )?
            builder.build()
        }
    };
    
    // Parse additional fields
    (@parse $builder:expr, $name:ident: $type:ident $(, $($rest:tt)*)?) => {
        {
            let mut builder = $builder.field(stringify!($name), schema!(@type $type));
            $(
                builder = schema!(@parse builder, $($rest)*);
            )?
            builder
        }
    };
    
    (@parse $builder:expr, $name:ident ?: $type:ident $(, $($rest:tt)*)?) => {
        {
            let mut builder = $builder.optional(stringify!($name), schema!(@type $type));
            $(
                builder = schema!(@parse builder, $($rest)*);
            )?
            builder
        }
    };
    
    // Type mappings
    (@type String) => { $crate::FieldType::String };
    (@type Number) => { $crate::FieldType::Number };
    (@type Boolean) => { $crate::FieldType::Boolean };
    (@type Date) => { $crate::FieldType::Date };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Schema, FieldType};
    use serde_json::json;

    #[test]
    fn test_simple_schema_macro() {
        let schema = schema! {
            name: String,
            age: Number,
            active: Boolean
        };
        
        assert_eq!(schema.fields.len(), 3);
        assert_eq!(schema.fields[0].key, "name");
        assert_eq!(schema.fields[0].field_type, FieldType::String);
        assert!(schema.fields[0].required);
        
        assert_eq!(schema.fields[1].key, "age");
        assert_eq!(schema.fields[1].field_type, FieldType::Number);
        assert!(schema.fields[1].required);
    }

    #[test]
    fn test_optional_fields_schema_macro() {
        let schema = schema! {
            name: String,
            age?: Number,
            email?: String
        };
        
        assert_eq!(schema.fields.len(), 3);
        assert!(schema.fields[0].required);
        assert!(!schema.fields[1].required);
        assert!(!schema.fields[2].required);
    }

    // Simplified tests for now - complex nested objects would need more macro work
    #[test]
    fn test_simple_validation_with_macro() {
        let schema = schema! {
            name: String,
            age?: Number
        };
        
        let valid_data = json!({
            "name": "John Doe",
            "age": 30
        });
        
        assert!(schema.validate(&valid_data).is_ok());
        
        let invalid_data = json!({
            "age": 30
            // missing required name
        });
        
        assert!(schema.validate(&invalid_data).is_err());
    }
}