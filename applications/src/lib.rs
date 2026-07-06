//! Synchronous application lifecycle management for ARIA.

use std::collections::BTreeMap;

/// Describes an ARIA application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationManifest {
    id: String,
    name: String,
    version: String,
    description: String,
}

impl ApplicationManifest {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: description.into(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

/// The lifecycle state of an application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationState {
    Registered,
    Initialized,
    Running,
    Suspended,
    Stopped,
}

/// An application hosted by ARIA.
pub trait Application {
    fn manifest(&self) -> &ApplicationManifest;
    fn initialize(&mut self);
    fn start(&mut self);
    fn stop(&mut self);
    fn suspend(&mut self);
    fn resume(&mut self);
}

struct ManagedApplication {
    application: Box<dyn Application>,
    state: ApplicationState,
}

/// Owns applications and coordinates their lifecycle.
#[derive(Default)]
pub struct ApplicationManager {
    applications: BTreeMap<String, ManagedApplication>,
}

impl ApplicationManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an application unless its manifest ID already exists.
    pub fn register(&mut self, application: impl Application + 'static) -> bool {
        let id = application.manifest().id().to_owned();

        if self.applications.contains_key(&id) {
            return false;
        }

        self.applications.insert(
            id,
            ManagedApplication {
                application: Box::new(application),
                state: ApplicationState::Registered,
            },
        );

        true
    }

    /// Initializes and starts an application.
    pub fn launch(&mut self, id: &str) -> bool {
        let Some(managed) = self.applications.get_mut(id) else {
            return false;
        };

        match managed.state {
            ApplicationState::Registered | ApplicationState::Stopped => {
                managed.application.initialize();
                managed.state = ApplicationState::Initialized;

                managed.application.start();
                managed.state = ApplicationState::Running;
                true
            }
            ApplicationState::Initialized => {
                managed.application.start();
                managed.state = ApplicationState::Running;
                true
            }
            ApplicationState::Running | ApplicationState::Suspended => false,
        }
    }

    /// Stops a running or suspended application.
    pub fn stop(&mut self, id: &str) -> bool {
        let Some(managed) = self.applications.get_mut(id) else {
            return false;
        };

        if !matches!(
            managed.state,
            ApplicationState::Running | ApplicationState::Suspended
        ) {
            return false;
        }

        managed.application.stop();
        managed.state = ApplicationState::Stopped;
        true
    }

    /// Suspends a running application.
    pub fn suspend(&mut self, id: &str) -> bool {
        let Some(managed) = self.applications.get_mut(id) else {
            return false;
        };

        if managed.state != ApplicationState::Running {
            return false;
        }

        managed.application.suspend();
        managed.state = ApplicationState::Suspended;
        true
    }

    /// Resumes a suspended application.
    pub fn resume(&mut self, id: &str) -> bool {
        let Some(managed) = self.applications.get_mut(id) else {
            return false;
        };

        if managed.state != ApplicationState::Suspended {
            return false;
        }

        managed.application.resume();
        managed.state = ApplicationState::Running;
        true
    }

    /// Lists applications in ascending manifest-ID order.
    pub fn list(&self) -> Vec<(&ApplicationManifest, ApplicationState)> {
        self.applications
            .values()
            .map(|managed| (managed.application.manifest(), managed.state))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Application, ApplicationManager, ApplicationManifest, ApplicationState};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct TestApplication {
        manifest: ApplicationManifest,
        calls: Rc<RefCell<Vec<&'static str>>>,
    }

    impl TestApplication {
        fn new(id: &str, calls: Rc<RefCell<Vec<&'static str>>>) -> Self {
            Self {
                manifest: ApplicationManifest::new(
                    id,
                    "Test Application",
                    "0.0.1",
                    "An application used for testing",
                ),
                calls,
            }
        }
    }

    impl Application for TestApplication {
        fn manifest(&self) -> &ApplicationManifest {
            &self.manifest
        }

        fn initialize(&mut self) {
            self.calls.borrow_mut().push("initialize");
        }

        fn start(&mut self) {
            self.calls.borrow_mut().push("start");
        }

        fn stop(&mut self) {
            self.calls.borrow_mut().push("stop");
        }

        fn suspend(&mut self) {
            self.calls.borrow_mut().push("suspend");
        }

        fn resume(&mut self) {
            self.calls.borrow_mut().push("resume");
        }
    }

    #[test]
    fn registers_and_lists_applications() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut manager = ApplicationManager::new();

        assert!(manager.register(TestApplication::new("compose", Rc::clone(&calls))));
        assert!(!manager.register(TestApplication::new("compose", calls)));

        let applications = manager.list();
        let (manifest, state) = applications[0];

        assert_eq!(applications.len(), 1);
        assert_eq!(manifest.id(), "compose");
        assert_eq!(manifest.name(), "Test Application");
        assert_eq!(manifest.version(), "0.0.1");
        assert_eq!(manifest.description(), "An application used for testing");
        assert_eq!(state, ApplicationState::Registered);
    }

    #[test]
    fn launches_an_application() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut manager = ApplicationManager::new();
        manager.register(TestApplication::new("compose", Rc::clone(&calls)));

        assert!(manager.launch("compose"));

        assert_eq!(*calls.borrow(), ["initialize", "start"]);
        assert_eq!(manager.list()[0].1, ApplicationState::Running);
    }

    #[test]
    fn suspends_and_resumes_an_application() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut manager = ApplicationManager::new();
        manager.register(TestApplication::new("compose", Rc::clone(&calls)));
        manager.launch("compose");

        assert!(manager.suspend("compose"));
        assert_eq!(manager.list()[0].1, ApplicationState::Suspended);

        assert!(manager.resume("compose"));
        assert_eq!(manager.list()[0].1, ApplicationState::Running);
        assert_eq!(
            *calls.borrow(),
            ["initialize", "start", "suspend", "resume"]
        );
    }

    #[test]
    fn stops_an_application() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut manager = ApplicationManager::new();
        manager.register(TestApplication::new("compose", Rc::clone(&calls)));
        manager.launch("compose");

        assert!(manager.stop("compose"));

        assert_eq!(manager.list()[0].1, ApplicationState::Stopped);
        assert_eq!(*calls.borrow(), ["initialize", "start", "stop"]);
    }

    #[test]
    fn rejects_unknown_applications_and_invalid_transitions() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut manager = ApplicationManager::new();
        manager.register(TestApplication::new("compose", calls));

        assert!(!manager.launch("missing"));
        assert!(!manager.stop("missing"));
        assert!(!manager.suspend("missing"));
        assert!(!manager.resume("missing"));

        assert!(!manager.stop("compose"));
        assert!(!manager.suspend("compose"));
        assert!(!manager.resume("compose"));

        assert!(manager.launch("compose"));
        assert!(!manager.launch("compose"));
        assert!(!manager.resume("compose"));

        assert!(manager.suspend("compose"));
        assert!(!manager.suspend("compose"));

        assert!(manager.stop("compose"));
        assert!(!manager.stop("compose"));
    }
}
