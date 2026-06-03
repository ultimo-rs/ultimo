//! Minimal fixture support for tests.

use serde::de::DeserializeOwned;
use std::future::Future;
use std::path::Path;

/// Load a typed fixture from a JSON file.
///
/// Panics with a clear message on failure — fixtures are test inputs, so a bad
/// path or malformed JSON is a test bug that should fail loudly.
pub fn load_fixture<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    let path = path.as_ref();
    let data = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()))
}

/// Optional setup/teardown lifecycle for seeding and cleaning test data.
pub trait Fixture {
    /// Seed data before a test.
    fn setup(&self) -> impl Future<Output = ()> + Send;
    /// Clean up after a test.
    fn teardown(&self) -> impl Future<Output = ()> + Send;
}
