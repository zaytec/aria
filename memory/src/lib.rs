//! Simple in-memory key-value storage for ARIA.

use std::collections::HashMap;

/// A synchronous in-memory store for string values.
#[derive(Default)]
pub struct Memory {
    entries: HashMap<String, String>,
}

impl Memory {
    /// Creates an empty memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a value and returns the previous value, if present.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.entries.insert(key.into(), value.into())
    }

    /// Retrieves a value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Deletes a value and returns it, if present.
    pub fn delete(&mut self, key: &str) -> Option<String> {
        self.entries.remove(key)
    }

    /// Returns whether a key exists.
    pub fn exists(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::Memory;

    #[test]
    fn stores_and_retrieves_values() {
        let mut memory = Memory::new();

        assert_eq!(memory.set("project.name", "ARIA"), None);
        assert_eq!(memory.get("project.name"), Some("ARIA"));
    }

    #[test]
    fn replaces_existing_values() {
        let mut memory = Memory::new();
        memory.set("project.name", "Original");

        assert_eq!(
            memory.set("project.name", "ARIA"),
            Some("Original".to_owned())
        );
        assert_eq!(memory.get("project.name"), Some("ARIA"));
    }

    #[test]
    fn deletes_values() {
        let mut memory = Memory::new();
        memory.set("project.name", "ARIA");

        assert_eq!(memory.delete("project.name"), Some("ARIA".to_owned()));
        assert!(!memory.exists("project.name"));
    }
}
