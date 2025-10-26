use std::collections::HashMap;
use crate::analyzer::{RouteInfo, ControllerInfo};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RouteTree {
    pub routes: Vec<RouteInfo>,
    pub conflicts: Vec<RouteConflict>,
    pub missing_handlers: Vec<String>,
    pub route_groups: HashMap<String, Vec<RouteInfo>>,
}

#[derive(Debug, Serialize)]
pub struct RouteConflict {
    pub method: String,
    pub path: String,
    pub conflicting_handlers: Vec<String>,
}

pub struct RouteAnalyzer;

impl RouteAnalyzer {
    pub fn build_route_tree(routes: &[RouteInfo], controllers: &[ControllerInfo]) -> RouteTree {
        let conflicts = Self::detect_conflicts(routes);
        let missing_handlers = Self::find_missing_handlers(routes, controllers);
        let route_groups = Self::group_routes_by_controller(routes, controllers);
        
        RouteTree {
            routes: routes.to_vec(),
            conflicts,
            missing_handlers,
            route_groups,
        }
    }
    
    fn detect_conflicts(routes: &[RouteInfo]) -> Vec<RouteConflict> {
        let mut conflicts = Vec::new();
        let mut route_map: HashMap<(String, String), Vec<String>> = HashMap::new();
        
        // Group routes by method and path
        for route in routes {
            let key = (route.method.clone(), route.path.clone());
            route_map.entry(key).or_default().push(route.handler.clone());
        }
        
        // Find conflicts (multiple handlers for same method/path)
        for ((method, path), handlers) in route_map {
            if handlers.len() > 1 {
                conflicts.push(RouteConflict {
                    method,
                    path,
                    conflicting_handlers: handlers,
                });
            }
        }
        
        conflicts
    }
    
    fn find_missing_handlers(routes: &[RouteInfo], controllers: &[ControllerInfo]) -> Vec<String> {
        let mut missing = Vec::new();
        let all_handlers: Vec<String> = controllers
            .iter()
            .flat_map(|c| c.handlers.iter().map(|h| h.name.clone()))
            .collect();
        
        for route in routes {
            if !all_handlers.contains(&route.handler) {
                missing.push(route.handler.clone());
            }
        }
        
        missing.sort();
        missing.dedup();
        missing
    }
    
    fn group_routes_by_controller(routes: &[RouteInfo], controllers: &[ControllerInfo]) -> HashMap<String, Vec<RouteInfo>> {
        let mut groups = HashMap::new();
        
        for controller in controllers {
            let controller_routes: Vec<RouteInfo> = routes
                .iter()
                .filter(|route| controller.handlers.iter().any(|h| h.name == route.handler))
                .map(|route| route.clone())
                .collect();
            
            if !controller_routes.is_empty() {
                groups.insert(controller.name.clone(), controller_routes);
            }
        }
        
        groups
    }
    
    pub fn analyze_route_parameters(routes: &[RouteInfo]) -> HashMap<String, Vec<String>> {
        let mut param_analysis = HashMap::new();
        
        for route in routes {
            if !route.parameters.is_empty() {
                param_analysis.insert(
                    format!("{} {}", route.method, route.path),
                    route.parameters.clone()
                );
            }
        }
        
        param_analysis
    }
    
    pub fn find_potential_conflicts(routes: &[RouteInfo]) -> Vec<(RouteInfo, RouteInfo)> {
        let mut potential_conflicts = Vec::new();
        
        for (i, route1) in routes.iter().enumerate() {
            for route2 in routes.iter().skip(i + 1) {
                if route1.method == route2.method && Self::paths_could_conflict(&route1.path, &route2.path) {
                    potential_conflicts.push((route1.clone(), route2.clone()));
                }
            }
        }
        
        potential_conflicts
    }
    
    fn paths_could_conflict(path1: &str, path2: &str) -> bool {
        // Simple conflict detection - more sophisticated logic could be added
        let segments1: Vec<&str> = path1.split('/').collect();
        let segments2: Vec<&str> = path2.split('/').collect();
        
        if segments1.len() != segments2.len() {
            return false;
        }
        
        for (seg1, seg2) in segments1.iter().zip(segments2.iter()) {
            // If both are parameters or both are exact matches, continue
            if (seg1.starts_with('{') && seg1.ends_with('}')) || 
               (seg2.starts_with('{') && seg2.ends_with('}')) {
                continue;
            }
            
            if seg1 != seg2 {
                return false;
            }
        }
        
        true
    }
}