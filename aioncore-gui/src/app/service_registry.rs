use crate::core::event_bus::EventHub;

/// A clonable container for all application services.
///
/// `ServiceRegistry` can be cheaply cloned and captured in async closures,
/// removing the need to access the GPUI global `AppState` from background tasks.
#[derive(Clone)]
pub struct ServiceRegistry {
    pub event_hub: EventHub,
}

impl ServiceRegistry {
    pub fn new(event_hub: EventHub) -> Self {
        Self { event_hub }
    }
}
