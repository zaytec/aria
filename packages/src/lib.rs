use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PackageId(String);

impl PackageId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PackageId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for PackageId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PackageVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl PackageVersion {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let mut parts = value.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;

        if parts.next().is_some() {
            return None;
        }

        Some(Self::new(major, minor, patch))
    }

    pub fn major(self) -> u64 {
        self.major
    }

    pub fn minor(self) -> u64 {
        self.minor
    }

    pub fn patch(self) -> u64 {
        self.patch
    }
}

impl fmt::Display for PackageVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl From<(u64, u64, u64)> for PackageVersion {
    fn from(value: (u64, u64, u64)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageManifest {
    id: PackageId,
    name: String,
    version: PackageVersion,
    dependencies: Vec<PackageId>,
}

impl PackageManifest {
    pub fn new(
        id: impl Into<PackageId>,
        name: impl Into<String>,
        version: PackageVersion,
        dependencies: Vec<PackageId>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version,
            dependencies,
        }
    }

    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> PackageVersion {
        self.version
    }

    pub fn dependencies(&self) -> &[PackageId] {
        &self.dependencies
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PackageState {
    #[default]
    Available,
    Installed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    manifest: PackageManifest,
    state: PackageState,
}

impl Package {
    pub fn new(manifest: PackageManifest) -> Self {
        Self {
            manifest,
            state: PackageState::Available,
        }
    }

    pub fn manifest(&self) -> &PackageManifest {
        &self.manifest
    }

    pub fn state(&self) -> PackageState {
        self.state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageError {
    AlreadyInstalled(PackageId),
    NotInstalled(PackageId),
    DependenciesMissing(Vec<PackageId>),
    RequiredBy(PackageId),
    VersionNotNewer {
        id: PackageId,
        installed: PackageVersion,
        requested: PackageVersion,
    },
}

#[derive(Default)]
pub struct PackageManager {
    packages: BTreeMap<PackageId, Package>,
}

impl PackageManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn install(&mut self, mut package: Package) -> Result<(), PackageError> {
        let id = package.manifest().id().clone();

        if self.exists(&id) {
            return Err(PackageError::AlreadyInstalled(id));
        }

        self.ensure_dependencies(package.manifest())?;
        package.state = PackageState::Installed;
        self.packages.insert(id, package);
        Ok(())
    }

    pub fn uninstall(&mut self, id: &PackageId) -> Result<Package, PackageError> {
        if !self.exists(id) {
            return Err(PackageError::NotInstalled(id.clone()));
        }

        if let Some(dependent) = self
            .packages
            .values()
            .find(|package| package.manifest().dependencies().contains(id))
        {
            return Err(PackageError::RequiredBy(dependent.manifest().id().clone()));
        }

        let mut package = self.packages.remove(id).expect("package existence checked");
        package.state = PackageState::Available;
        Ok(package)
    }

    pub fn update(&mut self, mut package: Package) -> Result<(), PackageError> {
        let id = package.manifest().id().clone();
        let Some(installed) = self.packages.get(&id) else {
            return Err(PackageError::NotInstalled(id));
        };

        if package.manifest().version() <= installed.manifest().version() {
            return Err(PackageError::VersionNotNewer {
                id,
                installed: installed.manifest().version(),
                requested: package.manifest().version(),
            });
        }

        self.ensure_dependencies(package.manifest())?;
        package.state = PackageState::Installed;
        self.packages.insert(id, package);
        Ok(())
    }

    pub fn list(&self) -> Vec<&Package> {
        self.packages.values().collect()
    }

    pub fn exists(&self, id: &PackageId) -> bool {
        self.packages.contains_key(id)
    }

    pub fn get(&self, id: &PackageId) -> Option<&Package> {
        self.packages.get(id)
    }

    pub fn validate_dependencies(&self, manifest: &PackageManifest) -> bool {
        self.missing_dependencies(manifest).is_empty()
    }

    fn ensure_dependencies(&self, manifest: &PackageManifest) -> Result<(), PackageError> {
        let missing = self.missing_dependencies(manifest);

        if missing.is_empty() {
            Ok(())
        } else {
            Err(PackageError::DependenciesMissing(missing))
        }
    }

    fn missing_dependencies(&self, manifest: &PackageManifest) -> Vec<PackageId> {
        manifest
            .dependencies()
            .iter()
            .filter(|dependency| !self.exists(dependency))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Package, PackageError, PackageId, PackageManager, PackageManifest, PackageState,
        PackageVersion,
    };

    fn package(id: &str, version: PackageVersion, dependencies: &[&str]) -> Package {
        Package::new(PackageManifest::new(
            id,
            format!("{id} package"),
            version,
            dependencies.iter().copied().map(PackageId::from).collect(),
        ))
    }

    #[test]
    fn compares_semantic_versions() {
        assert!(PackageVersion::new(2, 0, 0) > PackageVersion::new(1, 99, 99));
        assert!(PackageVersion::new(1, 2, 0) > PackageVersion::new(1, 1, 99));
        assert!(PackageVersion::new(1, 1, 1) > PackageVersion::new(1, 1, 0));
        assert_eq!(
            PackageVersion::parse("1.2.3"),
            Some(PackageVersion::new(1, 2, 3))
        );
        assert_eq!(PackageVersion::parse("1.2"), None);
    }

    #[test]
    fn installs_lists_and_retrieves_packages() {
        let mut manager = PackageManager::new();
        let id = PackageId::new("audio");

        manager
            .install(package("audio", PackageVersion::new(1, 0, 0), &[]))
            .unwrap();

        assert!(manager.exists(&id));
        assert_eq!(manager.list().len(), 1);
        assert_eq!(manager.get(&id).unwrap().state(), PackageState::Installed);
    }

    #[test]
    fn rejects_duplicate_packages_and_missing_dependencies() {
        let mut manager = PackageManager::new();
        let audio = package("audio", PackageVersion::new(1, 0, 0), &[]);
        manager.install(audio.clone()).unwrap();

        assert_eq!(
            manager.install(audio),
            Err(PackageError::AlreadyInstalled(PackageId::new("audio")))
        );
        assert_eq!(
            manager.install(package(
                "mixer",
                PackageVersion::new(1, 0, 0),
                &["audio", "dsp"]
            )),
            Err(PackageError::DependenciesMissing(vec![PackageId::new(
                "dsp"
            )]))
        );
    }

    #[test]
    fn validates_dependencies_and_prevents_removing_required_packages() {
        let mut manager = PackageManager::new();
        let audio = PackageId::new("audio");
        let mixer = package("mixer", PackageVersion::new(1, 0, 0), &["audio"]);

        assert!(!manager.validate_dependencies(mixer.manifest()));
        manager
            .install(package("audio", PackageVersion::new(1, 0, 0), &[]))
            .unwrap();
        assert!(manager.validate_dependencies(mixer.manifest()));
        manager.install(mixer).unwrap();

        assert_eq!(
            manager.uninstall(&audio),
            Err(PackageError::RequiredBy(PackageId::new("mixer")))
        );
    }

    #[test]
    fn updates_only_to_a_newer_version_and_uninstalls() {
        let mut manager = PackageManager::new();
        let id = PackageId::new("audio");
        manager
            .install(package("audio", PackageVersion::new(1, 0, 0), &[]))
            .unwrap();

        assert_eq!(
            manager.update(package("audio", PackageVersion::new(1, 0, 0), &[])),
            Err(PackageError::VersionNotNewer {
                id: id.clone(),
                installed: PackageVersion::new(1, 0, 0),
                requested: PackageVersion::new(1, 0, 0),
            })
        );
        manager
            .update(package("audio", PackageVersion::new(1, 1, 0), &[]))
            .unwrap();
        assert_eq!(
            manager.get(&id).unwrap().manifest().version(),
            PackageVersion::new(1, 1, 0)
        );

        let package = manager.uninstall(&id).unwrap();
        assert_eq!(package.state(), PackageState::Available);
        assert!(!manager.exists(&id));
    }
}
