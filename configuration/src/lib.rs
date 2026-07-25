use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationScope {
    System,
    User,
    Application,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigurationValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    List(Vec<ConfigurationValue>),
}

#[derive(Default)]
pub struct ConfigurationStore {
    namespaces: BTreeMap<String, BTreeMap<String, ConfigurationValue>>,
}

impl ConfigurationStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: ConfigurationValue,
    ) -> Option<ConfigurationValue> {
        self.namespaces
            .entry(namespace.into())
            .or_default()
            .insert(key.into(), value)
    }

    pub fn get(&self, namespace: &str, key: &str) -> Option<&ConfigurationValue> {
        self.namespaces.get(namespace)?.get(key)
    }

    pub fn remove(&mut self, namespace: &str, key: &str) -> Option<ConfigurationValue> {
        let values = self.namespaces.get_mut(namespace)?;
        let removed = values.remove(key);

        if values.is_empty() {
            self.namespaces.remove(namespace);
        }

        removed
    }

    pub fn exists(&self, namespace: &str, key: &str) -> bool {
        self.get(namespace, key).is_some()
    }

    pub fn list(&self, namespace: &str) -> Vec<(&str, &ConfigurationValue)> {
        self.namespaces
            .get(namespace)
            .into_iter()
            .flat_map(|values| values.iter())
            .map(|(key, value)| (key.as_str(), value))
            .collect()
    }

    pub fn namespaces(&self) -> Vec<&str> {
        self.namespaces.keys().map(String::as_str).collect()
    }
}

#[derive(Default)]
pub struct ConfigurationManager {
    defaults: ConfigurationStore,
    system: ConfigurationStore,
    user: ConfigurationStore,
    application: ConfigurationStore,
}

impl ConfigurationManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(
        &mut self,
        scope: ConfigurationScope,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: ConfigurationValue,
    ) -> Option<ConfigurationValue> {
        self.store_mut(scope).set(namespace, key, value)
    }

    pub fn set_default(
        &mut self,
        namespace: impl Into<String>,
        key: impl Into<String>,
        value: ConfigurationValue,
    ) -> Option<ConfigurationValue> {
        self.defaults.set(namespace, key, value)
    }

    pub fn get(&self, namespace: &str, key: &str) -> Option<&ConfigurationValue> {
        self.application
            .get(namespace, key)
            .or_else(|| self.user.get(namespace, key))
            .or_else(|| self.system.get(namespace, key))
            .or_else(|| self.defaults.get(namespace, key))
    }

    pub fn get_in(
        &self,
        scope: ConfigurationScope,
        namespace: &str,
        key: &str,
    ) -> Option<&ConfigurationValue> {
        self.store(scope).get(namespace, key)
    }

    pub fn remove(
        &mut self,
        scope: ConfigurationScope,
        namespace: &str,
        key: &str,
    ) -> Option<ConfigurationValue> {
        self.store_mut(scope).remove(namespace, key)
    }

    pub fn remove_default(&mut self, namespace: &str, key: &str) -> Option<ConfigurationValue> {
        self.defaults.remove(namespace, key)
    }

    pub fn exists(&self, namespace: &str, key: &str) -> bool {
        self.get(namespace, key).is_some()
    }

    pub fn exists_in(&self, scope: ConfigurationScope, namespace: &str, key: &str) -> bool {
        self.store(scope).exists(namespace, key)
    }

    pub fn list(
        &self,
        scope: ConfigurationScope,
        namespace: &str,
    ) -> Vec<(&str, &ConfigurationValue)> {
        self.store(scope).list(namespace)
    }

    pub fn list_resolved(&self, namespace: &str) -> Vec<(&str, &ConfigurationValue)> {
        let mut resolved = BTreeMap::new();

        for store in [&self.defaults, &self.system, &self.user, &self.application] {
            for (key, value) in store.list(namespace) {
                resolved.insert(key, value);
            }
        }

        resolved.into_iter().collect()
    }

    pub fn store(&self, scope: ConfigurationScope) -> &ConfigurationStore {
        match scope {
            ConfigurationScope::System => &self.system,
            ConfigurationScope::User => &self.user,
            ConfigurationScope::Application => &self.application,
        }
    }

    pub fn store_mut(&mut self, scope: ConfigurationScope) -> &mut ConfigurationStore {
        match scope {
            ConfigurationScope::System => &mut self.system,
            ConfigurationScope::User => &mut self.user,
            ConfigurationScope::Application => &mut self.application,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigurationManager, ConfigurationScope, ConfigurationStore, ConfigurationValue};

    #[test]
    fn stores_values_in_namespaces() {
        let mut store = ConfigurationStore::new();

        assert_eq!(
            store.set("audio", "volume", ConfigurationValue::Integer(80)),
            None
        );
        assert!(store.exists("audio", "volume"));
        assert_eq!(
            store.get("audio", "volume"),
            Some(&ConfigurationValue::Integer(80))
        );
        assert_eq!(store.list("audio").len(), 1);
        assert_eq!(store.namespaces(), ["audio"]);
        assert_eq!(
            store.remove("audio", "volume"),
            Some(ConfigurationValue::Integer(80))
        );
        assert!(!store.exists("audio", "volume"));
    }

    #[test]
    fn resolves_default_values_and_scope_inheritance() {
        let mut manager = ConfigurationManager::new();
        manager.set_default("display", "theme", ConfigurationValue::String("light".into()));
        manager.set(
            ConfigurationScope::System,
            "display",
            "theme",
            ConfigurationValue::String("dark".into()),
        );
        manager.set(
            ConfigurationScope::User,
            "display",
            "theme",
            ConfigurationValue::String("solarized".into()),
        );

        assert_eq!(
            manager.get("display", "theme"),
            Some(&ConfigurationValue::String("solarized".into()))
        );
        assert_eq!(
            manager.get_in(ConfigurationScope::System, "display", "theme"),
            Some(&ConfigurationValue::String("dark".into()))
        );
    }

    #[test]
    fn gives_application_values_precedence_and_falls_back_after_removal() {
        let mut manager = ConfigurationManager::new();
        manager.set_default("editor", "font_size", ConfigurationValue::Integer(12));
        manager.set(
            ConfigurationScope::User,
            "editor",
            "font_size",
            ConfigurationValue::Integer(14),
        );
        manager.set(
            ConfigurationScope::Application,
            "editor",
            "font_size",
            ConfigurationValue::Integer(16),
        );

        assert_eq!(
            manager.get("editor", "font_size"),
            Some(&ConfigurationValue::Integer(16))
        );
        assert!(manager.exists("editor", "font_size"));
        manager.remove(ConfigurationScope::Application, "editor", "font_size");
        assert_eq!(
            manager.get("editor", "font_size"),
            Some(&ConfigurationValue::Integer(14))
        );
    }

    #[test]
    fn lists_resolved_values_with_the_highest_priority_value() {
        let mut manager = ConfigurationManager::new();
        manager.set_default("audio", "volume", ConfigurationValue::Integer(50));
        manager.set_default("audio", "muted", ConfigurationValue::Boolean(false));
        manager.set(
            ConfigurationScope::Application,
            "audio",
            "volume",
            ConfigurationValue::Integer(75),
        );

        assert_eq!(
            manager.list_resolved("audio"),
            vec![
                ("muted", &ConfigurationValue::Boolean(false)),
                ("volume", &ConfigurationValue::Integer(75)),
            ]
        );
    }
}
