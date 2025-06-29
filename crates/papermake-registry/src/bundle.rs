use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata for a template containing descriptive information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateMetadata {
    /// Human-readable name of the template
    pub name: String,
    /// Author email or identifier
    pub author: String,
}

impl TemplateMetadata {
    /// Create new template metadata
    pub fn new(name: impl Into<String>, author: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            author: author.into(),
        }
    }

    /// Validate metadata fields
    pub fn validate(&self) -> Result<(), TemplateValidationError> {
        if self.name.trim().is_empty() {
            return Err(TemplateValidationError::InvalidMetadata(
                "Name cannot be empty".into(),
            ));
        }

        if self.author.trim().is_empty() {
            return Err(TemplateValidationError::InvalidMetadata(
                "Author cannot be empty".into(),
            ));
        }

        Ok(())
    }
}

/// A complete template bundle containing the main template file, metadata, and assets
#[derive(Debug, Clone)]
pub struct TemplateBundle {
    /// The main Typst template file content
    main_typ: Vec<u8>,
    /// Additional files (schema, assets, components, etc.)
    files: HashMap<String, Vec<u8>>,
    /// Template metadata
    metadata: TemplateMetadata,
}

impl TemplateBundle {
    /// Create a new template bundle with main.typ content and metadata
    pub fn new(main_typ: Vec<u8>, metadata: TemplateMetadata) -> Self {
        Self {
            main_typ,
            files: HashMap::new(),
            metadata,
        }
    }

    /// Add optional JSON schema for template data validation
    pub fn with_schema(mut self, schema_json: Vec<u8>) -> Self {
        self.files.insert("schema.json".to_string(), schema_json);
        self
    }

    /// Add an additional file (assets, components, etc.)
    pub fn add_file<P: AsRef<str>>(mut self, path: P, content: Vec<u8>) -> Self {
        self.files.insert(path.as_ref().to_string(), content);
        self
    }

    /// Get the main template content
    pub fn main_typ(&self) -> &[u8] {
        &self.main_typ
    }

    /// Get the main template content as a string
    pub fn main_typ_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.main_typ.clone())
    }

    /// Get all additional files
    pub fn files(&self) -> &HashMap<String, Vec<u8>> {
        &self.files
    }

    /// Get a specific file by path
    pub fn get_file(&self, path: &str) -> Option<&Vec<u8>> {
        self.files.get(path)
    }

    /// Get the template metadata
    pub fn metadata(&self) -> &TemplateMetadata {
        &self.metadata
    }

    /// Get mutable reference to metadata for updates
    pub fn metadata_mut(&mut self) -> &mut TemplateMetadata {
        &mut self.metadata
    }

    /// Check if the bundle has a schema file
    pub fn has_schema(&self) -> bool {
        self.files.contains_key("schema.json")
    }

    /// Get the schema content if it exists
    pub fn schema(&self) -> Option<&Vec<u8>> {
        self.files.get("schema.json")
    }

    /// List all file paths in the bundle
    pub fn file_paths(&self) -> Vec<&String> {
        self.files.keys().collect()
    }

    /// Get the total size of all files in bytes
    pub fn total_size(&self) -> usize {
        self.main_typ.len() + self.files.values().map(|v| v.len()).sum::<usize>()
    }

    /// Validate that the bundle is well-formed
    pub fn validate(&self) -> Result<(), TemplateValidationError> {
        // Check that main.typ is valid UTF-8
        String::from_utf8(self.main_typ.clone()).map_err(|_| {
            TemplateValidationError::InvalidMainTemplate("main.typ is not valid UTF-8".into())
        })?;

        // Check that metadata fields are not empty
        if self.metadata.name.trim().is_empty() {
            return Err(TemplateValidationError::InvalidMetadata(
                "name cannot be empty".into(),
            ));
        }

        if self.metadata.author.trim().is_empty() {
            return Err(TemplateValidationError::InvalidMetadata(
                "author cannot be empty".into(),
            ));
        }

        // Validate schema.json if present
        if let Some(schema_content) = self.schema() {
            serde_json::from_slice::<serde_json::Value>(schema_content).map_err(|e| {
                TemplateValidationError::InvalidSchema(format!("Invalid JSON schema: {}", e))
            })?;
        }

        Ok(())
    }
}

/// Errors that can occur during template bundle validation
#[derive(Debug, thiserror::Error)]
pub enum TemplateValidationError {
    #[error("Invalid main template: {0}")]
    InvalidMainTemplate(String),

    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),

    #[error("Invalid schema: {0}")]
    InvalidSchema(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metadata() -> TemplateMetadata {
        TemplateMetadata::new("Invoice Template", "john@example.com")
    }

    fn sample_template_content() -> Vec<u8> {
        br#"#let data = json.decode(sys.inputs.data)

= Invoice

*From:* #data.from
*To:* #data.to
*Date:* #data.date

== Items
#for item in data.items [
  - #item.description: #item.amount
]

*Total:* #data.total"#
            .to_vec()
    }

    #[test]
    fn test_template_metadata_creation() {
        let metadata = sample_metadata();

        assert_eq!(metadata.name, "Invoice Template");
        assert_eq!(metadata.author, "john@example.com");
    }

    #[test]
    fn test_template_bundle_creation() {
        let metadata = sample_metadata();
        let content = sample_template_content();
        let bundle = TemplateBundle::new(content.clone(), metadata.clone());

        assert_eq!(bundle.main_typ(), &content);
        assert_eq!(bundle.metadata(), &metadata);
        assert!(bundle.files().is_empty());
        assert!(!bundle.has_schema());
    }

    #[test]
    fn test_template_bundle_with_schema() {
        let metadata = sample_metadata();
        let content = sample_template_content();
        let schema = br#"{"type": "object", "properties": {"from": {"type": "string"}}}"#.to_vec();

        let bundle = TemplateBundle::new(content, metadata).with_schema(schema.clone());

        assert!(bundle.has_schema());
        assert_eq!(bundle.schema(), Some(&schema));
        assert_eq!(bundle.get_file("schema.json"), Some(&schema));
    }

    #[test]
    fn test_template_bundle_with_files() {
        let metadata = sample_metadata();
        let content = sample_template_content();
        let logo = b"fake_logo_data".to_vec();
        let header = b"#let header() = [Header Content]".to_vec();

        let bundle = TemplateBundle::new(content, metadata)
            .add_file("assets/logo.png", logo.clone())
            .add_file("components/header.typ", header.clone());

        assert_eq!(bundle.get_file("assets/logo.png"), Some(&logo));
        assert_eq!(bundle.get_file("components/header.typ"), Some(&header));
        assert_eq!(bundle.files().len(), 2);

        let mut paths = bundle.file_paths();
        paths.sort();
        assert_eq!(paths, vec!["assets/logo.png", "components/header.typ"]);
    }

    #[test]
    fn test_template_bundle_main_typ_string() {
        let metadata = sample_metadata();
        let content = sample_template_content();
        let bundle = TemplateBundle::new(content, metadata);

        let main_string = bundle.main_typ_string().unwrap();
        assert!(main_string.contains("#let data = json.decode(sys.inputs.data)"));
        assert!(main_string.contains("= Invoice"));
    }

    #[test]
    fn test_template_bundle_total_size() {
        let metadata = sample_metadata();
        let content = sample_template_content();
        let bundle = TemplateBundle::new(content.clone(), metadata)
            .add_file("test.txt", b"hello".to_vec())
            .add_file("test2.txt", b"world".to_vec());

        let expected_size = content.len() + 5 + 5; // main.typ + "hello" + "world"
        assert_eq!(bundle.total_size(), expected_size);
    }

    #[test]
    fn test_template_bundle_validation_success() {
        let metadata = sample_metadata();
        let content = sample_template_content();
        let schema = br#"{"type": "object"}"#.to_vec();

        let bundle = TemplateBundle::new(content, metadata).with_schema(schema);

        assert!(bundle.validate().is_ok());
    }

    #[test]
    fn test_template_bundle_validation_invalid_utf8() {
        let metadata = sample_metadata();
        let invalid_content = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8
        let bundle = TemplateBundle::new(invalid_content, metadata);

        let result = bundle.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TemplateValidationError::InvalidMainTemplate(_)
        ));
    }

    #[test]
    fn test_template_bundle_validation_empty_metadata() {
        let mut metadata = sample_metadata();
        metadata.name = "".to_string();
        let content = sample_template_content();
        let bundle = TemplateBundle::new(content, metadata);

        let result = bundle.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TemplateValidationError::InvalidMetadata(_)
        ));
    }

    #[test]
    fn test_template_bundle_validation_invalid_schema() {
        let metadata = sample_metadata();
        let content = sample_template_content();
        let invalid_schema = b"not valid json".to_vec();

        let bundle = TemplateBundle::new(content, metadata).with_schema(invalid_schema);

        let result = bundle.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TemplateValidationError::InvalidSchema(_)
        ));
    }
}
