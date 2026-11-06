//! The version that the binary reports.

/// Version of the running binary.
///
/// The value comes from `Cargo.toml` by default. A release build can set the
/// `TUICR_VERSION` environment variable at compile time. The value then
/// replaces the crate version. This lets a release tag name the version of the
/// released binary without a commit that changes `Cargo.toml`.
pub const fn version() -> &'static str {
    match option_env!("TUICR_VERSION") {
        Some(version) if !version.is_empty() => version,
        _ => env!("CARGO_PKG_VERSION"),
    }
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_not_empty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn version_has_no_v_prefix() {
        assert!(!version().starts_with('v'));
    }
}
