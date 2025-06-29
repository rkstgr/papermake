use crate::error::ReferenceError;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub namespace: Option<String>, // user or org
    pub name: String,              // "invoice"
    pub tag: Option<String>,       // "latest", "v1.0.0"
    pub hash: Option<String>,      // "sha256:abc123..."
}

impl Reference {
    /// Parse reference string: [namespace/]name[:tag][@hash]
    pub fn parse(reference: &str) -> Result<Self, ReferenceError> {
        if reference.is_empty() {
            return Err(ReferenceError::InvalidFormat {
                reference: reference.to_string(),
                reason: "Empty reference".to_string(),
            });
        }

        // Convert to lowercase for case insensitivity
        let reference = reference.to_lowercase();

        // Handle edge case: starts with @ (hash only)
        if reference.starts_with('@') {
            return Err(ReferenceError::InvalidFormat {
                reference: reference.clone(),
                reason: "Hash without namespace/name not allowed".to_string(),
            });
        }

        // Split by @ to separate hash
        let (main_part, hash) = if let Some(at_pos) = reference.rfind('@') {
            let hash_part = &reference[at_pos + 1..];
            if hash_part.is_empty() {
                return Err(ReferenceError::InvalidHash {
                    hash: "".to_string(),
                });
            }
            (reference[..at_pos].to_string(), Some(hash_part.to_string()))
        } else {
            (reference, None)
        };

        // Validate hash format if present
        if let Some(ref h) = hash {
            Self::validate_hash(h)?;
        }

        // Split by : to separate tag
        let (namespace_name_part, tag) = if let Some(colon_pos) = main_part.rfind(':') {
            let tag_part = &main_part[colon_pos + 1..];
            if tag_part.is_empty() {
                return Err(ReferenceError::InvalidTag {
                    tag: "".to_string(),
                    reason: "Empty tag not allowed".to_string(),
                });
            }
            (
                main_part[..colon_pos].to_string(),
                Some(tag_part.to_string()),
            )
        } else {
            // No tag specified, default to "latest"
            (main_part, Some("latest".to_string()))
        };

        // Validate tag format if present
        if let Some(ref t) = tag {
            Self::validate_tag(t)?;
        }

        // Split namespace and name by /
        let (namespace, name) = if let Some(slash_pos) = namespace_name_part.rfind('/') {
            let ns = namespace_name_part[..slash_pos].to_string();
            let n = namespace_name_part[slash_pos + 1..].to_string();
            (Some(ns), n)
        } else {
            (None, namespace_name_part)
        };

        // Validate namespace format if present
        if let Some(ref ns) = namespace {
            Self::validate_namespace(ns)?;
        }

        // Validate name format
        Self::validate_name(&name)?;

        Ok(Reference {
            namespace,
            name,
            tag,
            hash,
        })
    }

    /// Validate hash format (must start with sha256:)
    fn validate_hash(hash: &str) -> Result<(), ReferenceError> {
        if !hash.starts_with("sha256:") {
            return Err(ReferenceError::InvalidHash {
                hash: hash.to_string(),
            });
        }

        let hex_part = &hash[7..]; // Skip "sha256:"
        if hex_part.len() != 64 {
            return Err(ReferenceError::InvalidHash {
                hash: hash.to_string(),
            });
        }

        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ReferenceError::InvalidHash {
                hash: hash.to_string(),
            });
        }

        Ok(())
    }

    /// Validate namespace format (Docker registry rules: lowercase, alphanumeric, dots, dashes, underscores)
    fn validate_namespace(namespace: &str) -> Result<(), ReferenceError> {
        if namespace.is_empty() || namespace.len() > 255 {
            return Err(ReferenceError::InvalidNamespace {
                namespace: namespace.to_string(),
                reason: "Namespace length must be 1-255 characters".to_string(),
            });
        }

        // Docker registry naming rules
        let valid_chars = namespace.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-' || c == '_'
        });

        if !valid_chars {
            return Err(ReferenceError::InvalidNamespace {
                namespace: namespace.to_string(),
                reason: "Namespace can only contain lowercase letters, digits, dots, dashes, and underscores".to_string(),
            });
        }

        // Cannot start or end with special characters
        if namespace.starts_with('.')
            || namespace.starts_with('-')
            || namespace.starts_with('_')
            || namespace.ends_with('.')
            || namespace.ends_with('-')
            || namespace.ends_with('_')
        {
            return Err(ReferenceError::InvalidNamespace {
                namespace: namespace.to_string(),
                reason: "Namespace cannot start or end with '.', '-', or '_'".to_string(),
            });
        }

        Ok(())
    }

    /// Validate name format (same rules as namespace)
    fn validate_name(name: &str) -> Result<(), ReferenceError> {
        if name.is_empty() || name.len() > 255 {
            return Err(ReferenceError::InvalidFormat {
                reference: name.to_string(),
                reason: "Name length must be 1-255 characters".to_string(),
            });
        }

        let valid_chars = name.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-' || c == '_'
        });

        if !valid_chars {
            return Err(ReferenceError::InvalidFormat {
                reference: name.to_string(),
                reason:
                    "Name can only contain lowercase letters, digits, dots, dashes, and underscores"
                        .to_string(),
            });
        }

        if name.starts_with('.')
            || name.starts_with('-')
            || name.starts_with('_')
            || name.ends_with('.')
            || name.ends_with('-')
            || name.ends_with('_')
        {
            return Err(ReferenceError::InvalidFormat {
                reference: name.to_string(),
                reason: "Name cannot start or end with '.', '-', or '_'".to_string(),
            });
        }

        Ok(())
    }

    /// Validate tag format (semantic versioning or simple tags)
    fn validate_tag(tag: &str) -> Result<(), ReferenceError> {
        if tag.is_empty() || tag.len() > 128 {
            return Err(ReferenceError::InvalidTag {
                tag: tag.to_string(),
                reason: "Tag length must be 1-128 characters".to_string(),
            });
        }

        // Allow alphanumeric, dots, dashes, underscores
        let valid_chars = tag.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-' || c == '_'
        });

        if !valid_chars {
            return Err(ReferenceError::InvalidTag {
                tag: tag.to_string(),
                reason:
                    "Tag can only contain lowercase letters, digits, dots, dashes, and underscores"
                        .to_string(),
            });
        }

        Ok(())
    }

    /// Check if reference includes hash verification
    pub fn has_hash_verification(&self) -> bool {
        self.hash.is_some()
    }

    /// Get the full namespace/name path
    pub fn full_name(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{}/{}", ns, self.name),
            None => self.name.clone(),
        }
    }

    /// Get the tag, defaulting to "latest" if not specified
    pub fn tag_or_default(&self) -> &str {
        self.tag.as_deref().unwrap_or("latest")
    }

    /// Convert back to string representation  
    fn as_string(&self) -> String {
        let mut result = self.full_name();

        if let Some(ref tag) = self.tag {
            result.push(':');
            result.push_str(tag);
        }

        if let Some(ref hash) = self.hash {
            result.push('@');
            result.push_str(hash);
        }

        result
    }
}

impl FromStr for Reference {
    type Err = ReferenceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_name() {
        let ref_ = Reference::parse("invoice").unwrap();
        assert_eq!(ref_.namespace, None);
        assert_eq!(ref_.name, "invoice");
        assert_eq!(ref_.tag, Some("latest".to_string()));
        assert_eq!(ref_.hash, None);
    }

    #[test]
    fn test_parse_with_namespace() {
        let ref_ = Reference::parse("john/invoice").unwrap();
        assert_eq!(ref_.namespace, Some("john".to_string()));
        assert_eq!(ref_.name, "invoice");
        assert_eq!(ref_.tag, Some("latest".to_string()));
        assert_eq!(ref_.hash, None);
    }

    #[test]
    fn test_parse_with_tag() {
        let ref_ = Reference::parse("john/invoice:v1.0.0").unwrap();
        assert_eq!(ref_.namespace, Some("john".to_string()));
        assert_eq!(ref_.name, "invoice");
        assert_eq!(ref_.tag, Some("v1.0.0".to_string()));
        assert_eq!(ref_.hash, None);
    }

    #[test]
    fn test_parse_with_hash() {
        let ref_ = Reference::parse(
            "john/invoice@sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        )
        .unwrap();
        assert_eq!(ref_.namespace, Some("john".to_string()));
        assert_eq!(ref_.name, "invoice");
        assert_eq!(ref_.tag, Some("latest".to_string()));
        assert_eq!(
            ref_.hash,
            Some(
                "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_parse_full_reference() {
        let ref_ = Reference::parse("john/invoice:v1.0.0@sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        assert_eq!(ref_.namespace, Some("john".to_string()));
        assert_eq!(ref_.name, "invoice");
        assert_eq!(ref_.tag, Some("v1.0.0".to_string()));
        assert_eq!(
            ref_.hash,
            Some(
                "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_case_insensitive() {
        let ref_ = Reference::parse("John/Invoice:Latest").unwrap();
        assert_eq!(ref_.namespace, Some("john".to_string()));
        assert_eq!(ref_.name, "invoice");
        assert_eq!(ref_.tag, Some("latest".to_string()));
    }

    #[test]
    fn test_empty_tag_error() {
        assert!(matches!(
            Reference::parse("john/invoice:"),
            Err(ReferenceError::InvalidTag { .. })
        ));
    }

    #[test]
    fn test_hash_only_error() {
        assert!(matches!(
            Reference::parse("@sha256:abc123"),
            Err(ReferenceError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn test_invalid_hash_format() {
        assert!(matches!(
            Reference::parse("john/invoice@abc123"),
            Err(ReferenceError::InvalidHash { .. })
        ));
    }

    #[test]
    fn test_invalid_namespace_chars() {
        // Test actual invalid namespace characters
        assert!(matches!(
            Reference::parse("john$/invoice"),
            Err(ReferenceError::InvalidNamespace { .. })
        ));
    }

    #[test]
    fn test_invalid_hash_in_reference() {
        // This gets parsed as hash="/invoice" which is invalid
        assert!(matches!(
            Reference::parse("john@/invoice"),
            Err(ReferenceError::InvalidHash { .. })
        ));
    }

    #[test]
    fn test_to_string_roundtrip() {
        let original = "john/invoice:v1.0.0@sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let ref_ = Reference::parse(original).unwrap();
        assert_eq!(ref_.to_string(), original.to_lowercase());
    }
}
