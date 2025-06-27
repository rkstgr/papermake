//! Docker-style template references for papermake registry

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Docker-style template reference: [ORG/]NAME[:TAG][@DIGEST]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateRef {
    /// Optional organization namespace
    pub org: Option<String>,
    
    /// Template name (required)
    pub name: String,
    
    /// Version tag (defaults to "latest")
    pub tag: String,
    
    /// Optional content digest (SHA256 hash)
    pub digest: Option<String>,
}

impl TemplateRef {
    /// Create a new template reference
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            org: None,
            name: name.into(),
            tag: "latest".to_string(),
            digest: None,
        }
    }

    /// Create a new template reference with organization
    pub fn with_org(org: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            org: Some(org.into()),
            name: name.into(),
            tag: "latest".to_string(),
            digest: None,
        }
    }

    /// Set the tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Set the digest
    pub fn with_digest(mut self, digest: impl Into<String>) -> Self {
        self.digest = Some(digest.into());
        self
    }

    /// Get the name with optional organization prefix
    pub fn full_name(&self) -> String {
        match &self.org {
            Some(org) => format!("{}/{}", org, self.name),
            None => self.name.clone(),
        }
    }

    /// Get the name:tag portion
    pub fn name_tag(&self) -> String {
        format!("{}:{}", self.full_name(), self.tag)
    }

    /// Check if this references the latest tag
    pub fn is_latest(&self) -> bool {
        self.tag == "latest"
    }

    /// Check if this has a digest
    pub fn has_digest(&self) -> bool {
        self.digest.is_some()
    }

    /// Create a copy with a different tag
    pub fn with_different_tag(&self, tag: impl Into<String>) -> Self {
        Self {
            org: self.org.clone(),
            name: self.name.clone(),
            tag: tag.into(),
            digest: None, // Clear digest when changing tag
        }
    }

    /// Create a copy with a digest
    pub fn with_content_digest(&self, digest: impl Into<String>) -> Self {
        Self {
            org: self.org.clone(),
            name: self.name.clone(),
            tag: self.tag.clone(),
            digest: Some(digest.into()),
        }
    }
}

impl fmt::Display for TemplateRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = self.name_tag();
        
        if let Some(digest) = &self.digest {
            result.push_str(&format!("@{}", digest));
        }
        
        write!(f, "{}", result)
    }
}

impl FromStr for TemplateRef {
    type Err = TemplateRefParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split on @ to separate digest
        let (name_tag_part, digest) = match s.split_once('@') {
            Some((name_tag, digest)) => (name_tag, Some(digest.to_string())),
            None => (s, None),
        };

        // Split on : to separate tag
        let (name_part, tag) = match name_tag_part.rsplit_once(':') {
            Some((name, tag)) => (name, tag.to_string()),
            None => (name_tag_part, "latest".to_string()),
        };

        // Split on / to separate org and name
        let (org, name) = match name_part.split_once('/') {
            Some((org, name)) => (Some(org.to_string()), name.to_string()),
            None => (None, name_part.to_string()),
        };

        // Validate name is not empty
        if name.is_empty() {
            return Err(TemplateRefParseError::EmptyName);
        }

        // Validate tag is not empty
        if tag.is_empty() {
            return Err(TemplateRefParseError::EmptyTag);
        }

        Ok(TemplateRef {
            org,
            name,
            tag,
            digest,
        })
    }
}

/// Error type for parsing template references
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateRefParseError {
    EmptyName,
    EmptyTag,
}

impl fmt::Display for TemplateRefParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateRefParseError::EmptyName => write!(f, "Template name cannot be empty"),
            TemplateRefParseError::EmptyTag => write!(f, "Template tag cannot be empty"),
        }
    }
}

impl std::error::Error for TemplateRefParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_name() {
        let template_ref: TemplateRef = "invoice".parse().unwrap();
        assert_eq!(template_ref.org, None);
        assert_eq!(template_ref.name, "invoice");
        assert_eq!(template_ref.tag, "latest");
        assert_eq!(template_ref.digest, None);
        assert_eq!(template_ref.to_string(), "invoice:latest");
    }

    #[test]
    fn test_name_with_tag() {
        let template_ref: TemplateRef = "invoice:v1".parse().unwrap();
        assert_eq!(template_ref.org, None);
        assert_eq!(template_ref.name, "invoice");
        assert_eq!(template_ref.tag, "v1");
        assert_eq!(template_ref.digest, None);
        assert_eq!(template_ref.to_string(), "invoice:v1");
    }

    #[test]
    fn test_org_name_tag() {
        let template_ref: TemplateRef = "mycompany/invoice:v1".parse().unwrap();
        assert_eq!(template_ref.org, Some("mycompany".to_string()));
        assert_eq!(template_ref.name, "invoice");
        assert_eq!(template_ref.tag, "v1");
        assert_eq!(template_ref.digest, None);
        assert_eq!(template_ref.to_string(), "mycompany/invoice:v1");
    }

    #[test]
    fn test_name_tag_digest() {
        let template_ref: TemplateRef = "invoice:v1@sha256:abc123".parse().unwrap();
        assert_eq!(template_ref.org, None);
        assert_eq!(template_ref.name, "invoice");
        assert_eq!(template_ref.tag, "v1");
        assert_eq!(template_ref.digest, Some("sha256:abc123".to_string()));
        assert_eq!(template_ref.to_string(), "invoice:v1@sha256:abc123");
    }

    #[test]
    fn test_full_format() {
        let template_ref: TemplateRef = "mycompany/invoice:v1@sha256:abc123".parse().unwrap();
        assert_eq!(template_ref.org, Some("mycompany".to_string()));
        assert_eq!(template_ref.name, "invoice");
        assert_eq!(template_ref.tag, "v1");
        assert_eq!(template_ref.digest, Some("sha256:abc123".to_string()));
        assert_eq!(template_ref.to_string(), "mycompany/invoice:v1@sha256:abc123");
    }

    #[test]
    fn test_builder_methods() {
        let template_ref = TemplateRef::with_org("mycompany", "invoice")
            .with_tag("v2")
            .with_digest("sha256:def456");

        assert_eq!(template_ref.org, Some("mycompany".to_string()));
        assert_eq!(template_ref.name, "invoice");
        assert_eq!(template_ref.tag, "v2");
        assert_eq!(template_ref.digest, Some("sha256:def456".to_string()));
    }

    #[test]
    fn test_helper_methods() {
        let template_ref: TemplateRef = "mycompany/invoice:v1".parse().unwrap();
        assert_eq!(template_ref.full_name(), "mycompany/invoice");
        assert_eq!(template_ref.name_tag(), "mycompany/invoice:v1");
        assert!(!template_ref.is_latest());
        assert!(!template_ref.has_digest());

        let latest_ref: TemplateRef = "invoice".parse().unwrap();
        assert!(latest_ref.is_latest());
    }

    #[test]
    fn test_parse_errors() {
        assert!("".parse::<TemplateRef>().is_err());
        assert!(":v1".parse::<TemplateRef>().is_err());
        assert!("invoice:".parse::<TemplateRef>().is_err());
    }
}