use serde::Serialize;
use std::collections::HashMap;

use super::{FileChangeEvent, AffectedComponent, FileChangeType};

#[derive(Debug, Clone, Serialize)]
pub struct ChangeEventSummary {
    pub total_events: usize,
    pub event_types: HashMap<String, usize>,
    pub affected_controllers: Vec<String>,
    pub affected_routes: Vec<RouteChange>,
    pub affected_handlers: Vec<String>,
    pub affected_middleware: Vec<String>,
    pub affected_models: Vec<String>,
    pub affected_views: Vec<String>,
    pub config_changed: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteChange {
    pub method: String,
    pub path: String,
    pub change_type: String,
}

pub struct EventAggregator {
    events: Vec<FileChangeEvent>,
    window_duration: chrono::Duration,
}

impl EventAggregator {
    pub fn new(window_duration_secs: i64) -> Self {
        Self {
            events: Vec::new(),
            window_duration: chrono::Duration::seconds(window_duration_secs),
        }
    }
    
    pub fn add_event(&mut self, event: FileChangeEvent) {
        // Remove old events outside the window
        let cutoff = chrono::Utc::now() - self.window_duration;
        self.events.retain(|e| e.timestamp > cutoff);
        
        // Add new event
        self.events.push(event);
    }
    
    pub fn get_summary(&self) -> ChangeEventSummary {
        let mut event_types = HashMap::new();
        let mut affected_controllers = Vec::new();
        let mut affected_routes = Vec::new();
        let mut affected_handlers = Vec::new();
        let mut affected_middleware = Vec::new();
        let mut affected_models = Vec::new();
        let mut affected_views = Vec::new();
        let mut config_changed = false;
        
        for event in &self.events {
            // Count event types
            let event_type_str = match &event.event_type {
                FileChangeType::Created => "created",
                FileChangeType::Modified => "modified",
                FileChangeType::Deleted => "deleted",
                FileChangeType::Renamed { .. } => "renamed",
            };
            *event_types.entry(event_type_str.to_string()).or_insert(0) += 1;
            
            // Collect affected components
            for component in &event.affected_components {
                match component {
                    AffectedComponent::Controller { name } => {
                        if !affected_controllers.contains(name) {
                            affected_controllers.push(name.clone());
                        }
                    }
                    AffectedComponent::Route { method, path } => {
                        let route_change = RouteChange {
                            method: method.clone(),
                            path: path.clone(),
                            change_type: event_type_str.to_string(),
                        };
                        if !affected_routes.iter().any(|r: &RouteChange| r.method == *method && r.path == *path) {
                            affected_routes.push(route_change);
                        }
                    }
                    AffectedComponent::Handler { qualified_name } => {
                        if !affected_handlers.contains(qualified_name) {
                            affected_handlers.push(qualified_name.clone());
                        }
                    }
                    AffectedComponent::Middleware { name } => {
                        if !affected_middleware.contains(name) {
                            affected_middleware.push(name.clone());
                        }
                    }
                    AffectedComponent::Model { name } => {
                        if !affected_models.contains(name) {
                            affected_models.push(name.clone());
                        }
                    }
                    AffectedComponent::View { name } => {
                        if !affected_views.contains(name) {
                            affected_views.push(name.clone());
                        }
                    }
                    AffectedComponent::Config => {
                        config_changed = true;
                    }
                }
            }
        }
        
        ChangeEventSummary {
            total_events: self.events.len(),
            event_types,
            affected_controllers,
            affected_routes,
            affected_handlers,
            affected_middleware,
            affected_models,
            affected_views,
            config_changed,
            timestamp: chrono::Utc::now(),
        }
    }
    
    pub fn clear(&mut self) {
        self.events.clear();
    }
    
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}