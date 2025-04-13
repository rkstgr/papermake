//! Core library for the papermake PDF generation system

/// A simple function to verify the library is working
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// Public re-exports would go here in the future