//! Synchronous, type-safe event delivery for ARIA.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Marker trait for values that can be published through an [`EventBus`].
pub trait Event: Any + Send + Sync {}

impl<T> Event for T where T: Any + Send + Sync {}

type Handler = Box<dyn Fn(&dyn Any) + Send + Sync>;

/// A synchronous, type-safe event dispatcher.
///
/// Subscribers run immediately, in registration order, on the publishing thread.
#[derive(Default)]
pub struct EventBus {
    subscribers: HashMap<TypeId, Vec<Handler>>,
}

impl EventBus {
    /// Creates an empty event bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribes a handler to events of type `E`.
    pub fn subscribe<E: Event>(&mut self, handler: impl Fn(&E) + Send + Sync + 'static) {
        self.subscribers
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Box::new(move |event| {
                if let Some(event) = event.downcast_ref::<E>() {
                    handler(event);
                }
            }));
    }

    /// Publishes an event synchronously.
    ///
    /// Returns the number of handlers invoked.
    pub fn publish<E: Event>(&self, event: &E) -> usize {
        let Some(handlers) = self.subscribers.get(&TypeId::of::<E>()) else {
            return 0;
        };

        for handler in handlers {
            handler(event);
        }

        handlers.len()
    }
}

/// Indicates that a project has been created.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCreated {
    name: String,
}

impl ProjectCreated {
    /// Creates a project-created event.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Returns the created project's name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::{EventBus, ProjectCreated};
    use std::sync::{Arc, Mutex};

    #[test]
    fn publishes_an_event_to_its_subscribers() {
        let received_name = Arc::new(Mutex::new(None));
        let captured_name = Arc::clone(&received_name);
        let mut bus = EventBus::new();

        bus.subscribe::<ProjectCreated>(move |event| {
            *captured_name.lock().unwrap() = Some(event.name().to_owned());
        });

        let subscriber_count = bus.publish(&ProjectCreated::new("ARIA"));

        assert_eq!(subscriber_count, 1);
        assert_eq!(received_name.lock().unwrap().as_deref(), Some("ARIA"));
    }

    #[test]
    fn ignores_unsubscribed_event_types() {
        struct RuntimeStarted;

        let bus = EventBus::new();

        assert_eq!(bus.publish(&RuntimeStarted), 0);
    }
}
