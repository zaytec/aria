use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ServiceId(String);

impl ServiceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ServiceState {
    #[default]
    Registered,
    Initialized,
    Running,
    Stopped,
}

pub trait Service {
    fn id(&self) -> &ServiceId;
    fn name(&self) -> &str;
    fn initialize(&mut self);
    fn start(&mut self);
    fn stop(&mut self);
    fn restart(&mut self);
    fn state(&self) -> ServiceState;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServiceError {
    AlreadyRegistered(ServiceId),
    NotFound(ServiceId),
    InvalidState {
        id: ServiceId,
        operation: &'static str,
        state: ServiceState,
    },
}

#[derive(Default)]
pub struct ServiceManager {
    services: BTreeMap<ServiceId, Box<dyn Service>>,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, service: impl Service + 'static) -> Result<(), ServiceError> {
        let id = service.id().clone();

        if self.services.contains_key(&id) {
            return Err(ServiceError::AlreadyRegistered(id));
        }

        self.services.insert(id, Box::new(service));
        Ok(())
    }

    pub fn start(&mut self, id: &ServiceId) -> Result<(), ServiceError> {
        let service = self.service_mut(id)?;

        match service.state() {
            ServiceState::Registered => {
                service.initialize();
                Self::require_state(service, id, "initialize", ServiceState::Initialized)?;
                service.start();
                Self::require_state(service, id, "start", ServiceState::Running)
            }
            ServiceState::Initialized | ServiceState::Stopped => {
                service.start();
                Self::require_state(service, id, "start", ServiceState::Running)
            }
            state => Err(Self::invalid_state(id, "start", state)),
        }
    }

    pub fn stop(&mut self, id: &ServiceId) -> Result<(), ServiceError> {
        let service = self.service_mut(id)?;

        if service.state() != ServiceState::Running {
            return Err(Self::invalid_state(id, "stop", service.state()));
        }

        service.stop();
        Self::require_state(service, id, "stop", ServiceState::Stopped)
    }

    pub fn restart(&mut self, id: &ServiceId) -> Result<(), ServiceError> {
        let service = self.service_mut(id)?;

        if service.state() != ServiceState::Running {
            return Err(Self::invalid_state(id, "restart", service.state()));
        }

        service.restart();
        Self::require_state(service, id, "restart", ServiceState::Running)
    }

    pub fn get(&self, id: &ServiceId) -> Option<&dyn Service> {
        self.services.get(id).map(Box::as_ref)
    }

    pub fn list(&self) -> Vec<&dyn Service> {
        self.services.values().map(Box::as_ref).collect()
    }

    fn service_mut(
        &mut self,
        id: &ServiceId,
    ) -> Result<&mut (dyn Service + 'static), ServiceError> {
        self.services
            .get_mut(id)
            .map(Box::as_mut)
            .ok_or_else(|| ServiceError::NotFound(id.clone()))
    }

    fn require_state(
        service: &dyn Service,
        id: &ServiceId,
        operation: &'static str,
        expected: ServiceState,
    ) -> Result<(), ServiceError> {
        if service.state() == expected {
            Ok(())
        } else {
            Err(Self::invalid_state(id, operation, service.state()))
        }
    }

    fn invalid_state(id: &ServiceId, operation: &'static str, state: ServiceState) -> ServiceError {
        ServiceError::InvalidState {
            id: id.clone(),
            operation,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Service, ServiceError, ServiceId, ServiceManager, ServiceState};

    struct TestService {
        id: ServiceId,
        name: String,
        state: ServiceState,
        calls: Vec<&'static str>,
    }

    impl TestService {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: ServiceId::new(id),
                name: name.to_owned(),
                state: ServiceState::Registered,
                calls: Vec::new(),
            }
        }
    }

    impl Service for TestService {
        fn id(&self) -> &ServiceId {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn initialize(&mut self) {
            self.calls.push("initialize");
            self.state = ServiceState::Initialized;
        }

        fn start(&mut self) {
            self.calls.push("start");
            self.state = ServiceState::Running;
        }

        fn stop(&mut self) {
            self.calls.push("stop");
            self.state = ServiceState::Stopped;
        }

        fn restart(&mut self) {
            self.calls.push("restart");
            self.state = ServiceState::Running;
        }

        fn state(&self) -> ServiceState {
            self.state
        }
    }

    #[test]
    fn starts_a_registered_service_through_initialization() {
        let mut manager = ServiceManager::new();
        let id = ServiceId::new("audio");
        manager
            .register(TestService::new("audio", "Audio"))
            .unwrap();

        manager.start(&id).unwrap();

        assert_eq!(manager.get(&id).unwrap().state(), ServiceState::Running);
    }

    #[test]
    fn manages_multiple_registered_services() {
        let mut manager = ServiceManager::new();
        let audio = ServiceId::new("audio");
        let network = ServiceId::new("network");
        manager
            .register(TestService::new("audio", "Audio"))
            .unwrap();
        manager
            .register(TestService::new("network", "Network"))
            .unwrap();

        manager.start(&audio).unwrap();

        assert_eq!(manager.list().len(), 2);
        assert_eq!(manager.get(&network).unwrap().name(), "Network");
        assert_eq!(
            manager.get(&network).unwrap().state(),
            ServiceState::Registered
        );
    }

    #[test]
    fn stops_and_restarts_a_running_service() {
        let mut manager = ServiceManager::new();
        let id = ServiceId::new("audio");
        manager
            .register(TestService::new("audio", "Audio"))
            .unwrap();
        manager.start(&id).unwrap();

        manager.stop(&id).unwrap();
        assert_eq!(manager.get(&id).unwrap().state(), ServiceState::Stopped);

        manager.start(&id).unwrap();
        manager.restart(&id).unwrap();
        assert_eq!(manager.get(&id).unwrap().state(), ServiceState::Running);
    }

    #[test]
    fn rejects_duplicate_services_and_invalid_transitions() {
        let mut manager = ServiceManager::new();
        let id = ServiceId::new("audio");
        manager
            .register(TestService::new("audio", "Audio"))
            .unwrap();

        assert_eq!(
            manager.register(TestService::new("audio", "Replacement")),
            Err(ServiceError::AlreadyRegistered(id.clone()))
        );
        assert_eq!(
            manager.stop(&id),
            Err(ServiceError::InvalidState {
                id: id.clone(),
                operation: "stop",
                state: ServiceState::Registered,
            })
        );
        assert_eq!(
            manager.restart(&id),
            Err(ServiceError::InvalidState {
                id,
                operation: "restart",
                state: ServiceState::Registered,
            })
        );
    }

    #[test]
    fn rejects_unknown_services() {
        let mut manager = ServiceManager::new();
        let id = ServiceId::new("missing");

        assert_eq!(manager.start(&id), Err(ServiceError::NotFound(id)));
    }
}
