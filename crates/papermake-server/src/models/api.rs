//! Common API types and utilities
use serde::{Deserialize, Serialize};

/// Standard pagination parameters
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,

    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    50
}

/// Standard pagination response
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

/// Pagination metadata
#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub limit: u32,
    pub offset: u32,
    pub total: Option<u32>,
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, limit: u32, offset: u32, total: Option<u32>) -> Self {
        let has_more = match total {
            Some(t) => offset + (data.len() as u32) < t,
            None => data.len() as u32 == limit,
        };

        Self {
            data,
            pagination: PaginationInfo {
                limit,
                offset,
                total,
                has_more,
            },
        }
    }
}

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            message: None,
        }
    }

    pub fn with_message(data: T, message: String) -> Self {
        Self {
            data,
            message: Some(message),
        }
    }
}

/// Common query parameters for filtering
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    #[serde(flatten)]
    pub pagination: PaginationQuery,

    /// Search term for name/content filtering
    #[allow(dead_code)]
    pub search: Option<String>,

    /// Sort field
    #[allow(dead_code)]
    pub sort_by: Option<String>,

    /// Sort direction
    #[allow(dead_code)]
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Desc
    }
}
