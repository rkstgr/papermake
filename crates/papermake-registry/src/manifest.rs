use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

use crate::bundle::TemplateMetadata;

/// Template manifest containing file hashes and metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    /// Entry point file (always "main.typ")
    pub entrypoint: String,

    /// Map of file paths to their content hashes
    /// main.typ -> sha256:<hash>
    pub files: BTreeMap<String, String>,

    /// Template metadata
    pub metadata: TemplateMetadata,
}

impl Manifest {
    pub fn new(
        files: BTreeMap<String, String>,
        metadata: TemplateMetadata,
    ) -> Result<Self, ManifestError> {
        let entrypoint = "main.typ".to_string();

        // Validate that main.typ exists in files
        if !files.contains_key(&entrypoint) {
            return Err(ManifestError::MissingEntrypoint);
        }

        // Validate metadata
        metadata
            .validate()
            .map_err(|err| ManifestError::InvalidMetadata(err.to_string()))?;

        // Validate file hashes
        for (path, hash) in &files {
            Self::validate_file_path(path)?;
            Self::validate_hash(hash)?;
        }

        Ok(Self {
            entrypoint,
            files,
            metadata,
        })
    }

    /// Get the hash of the entry point file
    pub fn entrypoint_hash(&self) -> Option<&String> {
        self.files.get(&self.entrypoint)
    }

    /// Get all file paths in the manifest
    pub fn file_paths(&self) -> Vec<&String> {
        self.files.keys().collect()
    }

    /// Check if manifest contains a specific file
    pub fn has_file(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Get hash for a specific file
    pub fn get_file_hash(&self, path: &str) -> Option<&String> {
        self.files.get(path)
    }

    /// Add or update a file in the manifest
    pub fn add_file(&mut self, path: String, hash: String) -> Result<(), ManifestError> {
        Self::validate_file_path(&path)?;
        Self::validate_hash(&hash)?;
        self.files.insert(path, hash);
        Ok(())
    }

    /// Remove a file from the manifest
    pub fn remove_file(&mut self, path: &str) -> Option<String> {
        // Prevent removal of entrypoint
        if path == self.entrypoint {
            return None;
        }
        self.files.remove(path)
    }

    /// Serialize manifest to JSON bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, ManifestError> {
        serde_json::to_vec_pretty(self).map_err(ManifestError::Serialization)
    }

    /// Deserialize manifest from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ManifestError> {
        let manifest: Manifest =
            serde_json::from_slice(bytes).map_err(ManifestError::Serialization)?;

        // Re-validate after deserialization
        manifest.validate()?;

        Ok(manifest)
    }

    /// Validate the entire manifest
    pub fn validate(&self) -> Result<(), ManifestError> {
        // Check entrypoint exists
        if !self.files.contains_key(&self.entrypoint) {
            return Err(ManifestError::MissingEntrypoint);
        }

        // Validate metadata
        self.metadata
            .validate()
            .map_err(|err| ManifestError::InvalidMetadata(err.to_string()))?;

        // Validate all files
        for (path, hash) in &self.files {
            Self::validate_file_path(path)?;
            Self::validate_hash(hash)?;
        }

        Ok(())
    }

    /// Validate file path format
    fn validate_file_path(path: &str) -> Result<(), ManifestError> {
        if path.trim().is_empty() {
            return Err(ManifestError::InvalidFilePath(
                "Path cannot be empty".into(),
            ));
        }

        // Prevent path traversal attacks
        if path.contains("..") {
            return Err(ManifestError::InvalidFilePath(
                "Path cannot contain '..' segments".into(),
            ));
        }

        // Prevent absolute paths
        if path.starts_with('/') {
            return Err(ManifestError::InvalidFilePath(
                "Path cannot be absolute".into(),
            ));
        }

        Ok(())
    }

    /// Validate hash format (SHA-256)
    fn validate_hash(hash: &str) -> Result<(), ManifestError> {
        if !hash.starts_with("sha256:") {
            return Err(ManifestError::InvalidHash(
                "Hash must start with 'sha256:'".into(),
            ));
        }

        let hex_part = &hash[7..]; // Remove "sha256:" prefix
        if hex_part.len() != 64 {
            return Err(ManifestError::InvalidHash(
                "SHA-256 hash must be 64 hex characters".into(),
            ));
        }

        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ManifestError::InvalidHash(
                "Hash must contain only hexadecimal characters".into(),
            ));
        }

        Ok(())
    }
}

/// Errors that can occur when working with manifests
#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("Missing entrypoint file (main.typ)")]
    MissingEntrypoint,

    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),

    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),

    #[error("Invalid hash format: {0}")]
    InvalidHash(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use crate::bundle::TemplateValidationError;

    use super::*;

    fn create_test_metadata() -> TemplateMetadata {
        TemplateMetadata::new("Test Template".to_string(), "test@example.com".to_string())
    }

    fn create_test_files() -> BTreeMap<String, String> {
        let mut files = BTreeMap::new();
        files.insert(
            "main.typ".to_string(),
            "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        );
        files.insert(
            "schema.json".to_string(),
            "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        );
        files
    }

    #[test]
    fn test_manifest_creation() {
        let metadata = create_test_metadata();
        let files = create_test_files();

        let manifest = Manifest::new(files, metadata).unwrap();

        assert_eq!(manifest.entrypoint, "main.typ");
        assert_eq!(manifest.files.len(), 2);
        assert!(manifest.has_file("main.typ"));
        assert!(manifest.has_file("schema.json"));
    }

    #[test]
    fn test_manifest_missing_entrypoint() {
        let metadata = create_test_metadata();
        let mut files = BTreeMap::new();
        files.insert(
            "other.typ".to_string(),
            "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        );

        let result = Manifest::new(files, metadata);
        assert!(matches!(result, Err(ManifestError::MissingEntrypoint)));
    }

    #[test]
    fn test_metadata_validation() {
        let mut metadata = create_test_metadata();
        metadata.name = "".to_string();

        let result = metadata.validate();
        assert!(matches!(
            result,
            Err(TemplateValidationError::InvalidMetadata(_))
        ));
    }

    #[test]
    fn test_file_path_validation() {
        assert!(Manifest::validate_file_path("valid/path.typ").is_ok());
        assert!(Manifest::validate_file_path("../invalid").is_err());
        assert!(Manifest::validate_file_path("/absolute").is_err());
        assert!(Manifest::validate_file_path("").is_err());
    }

    #[test]
    fn test_hash_validation() {
        assert!(
            Manifest::validate_hash(
                "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            )
            .is_ok()
        );
        assert!(Manifest::validate_hash("invalid-hash").is_err());
        assert!(Manifest::validate_hash("sha256:short").is_err());
        assert!(Manifest::validate_hash("sha256:xyz").is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let metadata = create_test_metadata();
        let files = create_test_files();
        let manifest = Manifest::new(files, metadata).unwrap();

        let bytes = manifest.to_bytes().unwrap();
        let deserialized = Manifest::from_bytes(&bytes).unwrap();

        assert_eq!(manifest, deserialized);
    }

    #[test]
    fn test_file_operations() {
        let metadata = create_test_metadata();
        let files = create_test_files();
        let mut manifest = Manifest::new(files, metadata).unwrap();

        // Add file
        manifest
            .add_file(
                "assets/logo.png".to_string(),
                "sha256:fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"
                    .to_string(),
            )
            .unwrap();

        assert!(manifest.has_file("assets/logo.png"));

        // Remove file (not entrypoint)
        let removed = manifest.remove_file("assets/logo.png");
        assert!(removed.is_some());
        assert!(!manifest.has_file("assets/logo.png"));

        // Cannot remove entrypoint
        let removed = manifest.remove_file("main.typ");
        assert!(removed.is_none());
        assert!(manifest.has_file("main.typ"));
    }
}
