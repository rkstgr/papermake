use sha2::{Digest, Sha256};

/// Utilities for content-addressable storage using SHA-256 hashing
pub struct ContentAddress;

impl ContentAddress {
    /// Generate SHA-256 hash of content, returns hash with "sha256:" prefix
    pub fn hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        format!("sha256:{:x}", result)
    }

    /// Generate storage key for blob content
    /// Example: "blobs/sha256/abc123def456..."
    pub fn blob_key(hash: &str) -> String {
        let hash_value = Self::extract_hash_value(hash);
        format!("blobs/sha256/{}", hash_value)
    }

    /// Generate storage key for manifest
    /// Example: "manifests/sha256/abc123def456..."
    pub fn manifest_key(hash: &str) -> String {
        let hash_value = Self::extract_hash_value(hash);
        format!("manifests/sha256/{}", hash_value)
    }

    /// Generate storage key for reference (tag)
    /// Example: "refs/john/invoice/latest" or "refs/invoice/latest"
    pub fn ref_key(namespace: &str, tag: &str) -> String {
        format!("refs/{}/{}", namespace, tag)
    }

    /// Generate storage key for render input data
    /// Example: "data/sha256/abc123def456..."
    pub fn data_key(hash: &str) -> String {
        let hash_value = Self::extract_hash_value(hash);
        format!("data/sha256/{}", hash_value)
    }

    /// Generate storage key for rendered PDF
    /// Example: "pdfs/sha256/abc123def456..."
    pub fn pdf_key(hash: &str) -> String {
        let hash_value = Self::extract_hash_value(hash);
        format!("pdfs/sha256/{}", hash_value)
    }

    /// Extract hash value from full hash string (removes "sha256:" prefix)
    /// Example: "sha256:abc123..." -> "abc123..."
    pub fn extract_hash_value(hash: &str) -> &str {
        hash.strip_prefix("sha256:").unwrap_or(hash)
    }

    /// Validate that a hash string has the correct format
    pub fn is_valid_hash(hash: &str) -> bool {
        if !hash.starts_with("sha256:") {
            return false;
        }

        let hash_value = &hash[7..]; // Skip "sha256:" prefix

        // SHA-256 produces 64 hex characters
        hash_value.len() == 64 && hash_value.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Verify content matches expected hash
    pub fn verify(content: &[u8], expected_hash: &str) -> bool {
        let actual_hash = Self::hash(content);
        actual_hash == expected_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_generation() {
        let content = b"hello world";
        let hash = ContentAddress::hash(content);

        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71); // "sha256:" (7) + 64 hex chars

        // Same content should produce same hash
        let hash2 = ContentAddress::hash(content);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_different_content_different_hash() {
        let hash1 = ContentAddress::hash(b"hello");
        let hash2 = ContentAddress::hash(b"world");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_blob_key_generation() {
        let hash = "sha256:abc123def456789";
        let key = ContentAddress::blob_key(&hash);

        assert_eq!(key, "blobs/sha256/abc123def456789");
    }

    #[test]
    fn test_manifest_key_generation() {
        let hash = "sha256:abc123def456789";
        let key = ContentAddress::manifest_key(&hash);

        assert_eq!(key, "manifests/sha256/abc123def456789");
    }

    #[test]
    fn test_ref_key_generation() {
        // User template
        let key = ContentAddress::ref_key("john/invoice", "latest");
        assert_eq!(key, "refs/john/invoice/latest");

        // Official template
        let key = ContentAddress::ref_key("invoice", "latest");
        assert_eq!(key, "refs/invoice/latest");

        // Org template
        let key = ContentAddress::ref_key("acme-corp/letterhead", "stable");
        assert_eq!(key, "refs/acme-corp/letterhead/stable");
    }

    #[test]
    fn test_data_key_generation() {
        let hash = "sha256:abc123def456789";
        let key = ContentAddress::data_key(&hash);
        assert_eq!(key, "data/sha256/abc123def456789");
    }

    #[test]
    fn test_pdf_key_generation() {
        let hash = "sha256:abc123def456789";
        let key = ContentAddress::pdf_key(&hash);
        assert_eq!(key, "pdfs/sha256/abc123def456789");
    }

    #[test]
    fn test_extract_hash_value() {
        let hash = "sha256:abc123def456";
        let value = ContentAddress::extract_hash_value(&hash);
        assert_eq!(value, "abc123def456");

        // Should work with hash without prefix too
        let value = ContentAddress::extract_hash_value("abc123def456");
        assert_eq!(value, "abc123def456");
    }

    #[test]
    fn test_hash_validation() {
        // Valid hash
        let valid_hash = "sha256:".to_string() + &"a".repeat(64);
        assert!(ContentAddress::is_valid_hash(&valid_hash));

        // Invalid: no prefix
        assert!(!ContentAddress::is_valid_hash(&"a".repeat(64)));

        // Invalid: wrong length
        assert!(!ContentAddress::is_valid_hash("sha256:abc123"));

        // Invalid: non-hex characters
        assert!(!ContentAddress::is_valid_hash(
            &("sha256:".to_string() + &"g".repeat(64))
        ));

        // Invalid: too long
        assert!(!ContentAddress::is_valid_hash(
            &("sha256:".to_string() + &"a".repeat(65))
        ));
    }

    #[test]
    fn test_verify_content() {
        let content = b"test content";
        let hash = ContentAddress::hash(content);

        // Should verify correctly
        assert!(ContentAddress::verify(content, &hash));

        // Should fail with wrong content
        assert!(!ContentAddress::verify(b"wrong content", &hash));

        // Should fail with wrong hash
        assert!(!ContentAddress::verify(content, "sha256:wrong_hash"));
    }

    #[test]
    fn test_real_sha256_values() {
        // Test with known SHA-256 values
        let content = b"hello";
        let hash = ContentAddress::hash(content);

        // "hello" should hash to this specific value
        assert_eq!(
            hash,
            "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
