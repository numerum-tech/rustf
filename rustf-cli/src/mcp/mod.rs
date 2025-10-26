use std::path::PathBuf;
use anyhow::Result;
use tokio::sync::{RwLock, mpsc};
use std::sync::Arc;

mod server;
mod tools;
mod resources;
mod watcher;
mod instance_registry;
mod interface;
mod cli_executor;

pub use server::McpServer;
pub use instance_registry::{ServerInstance, get_registry};

use crate::analyzer::ProjectAnalyzer;
use crate::watcher::{ProjectWatcher, FileChangeEvent, EventAggregator};

/// Core MCP server state
pub struct McpState {
    pub project_path: PathBuf,
    pub analyzer: Arc<RwLock<ProjectAnalyzer>>,
    pub watch_enabled: bool,
    pub watcher: Option<Arc<ProjectWatcher>>,
    pub event_receiver: Option<mpsc::UnboundedReceiver<FileChangeEvent>>,
    pub event_aggregator: Arc<RwLock<EventAggregator>>,
    pub connected_clients: Arc<RwLock<Vec<String>>>, // Client IDs for notifications
}

impl McpState {
    pub fn new(project_path: PathBuf, watch_enabled: bool) -> Result<Self> {
        let analyzer = ProjectAnalyzer::new(project_path.clone())?;
        let analyzer_arc = Arc::new(RwLock::new(analyzer));
        
        let (watcher, event_receiver) = if watch_enabled {
            let (watcher, receiver) = ProjectWatcher::new(project_path.clone(), analyzer_arc.clone())?;
            (Some(Arc::new(watcher)), Some(receiver))
        } else {
            (None, None)
        };
        
        Ok(Self {
            project_path,
            analyzer: analyzer_arc,
            watch_enabled,
            watcher,
            event_receiver,
            event_aggregator: Arc::new(RwLock::new(EventAggregator::new(5))), // 5-second window
            connected_clients: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    pub async fn start_file_watcher(&mut self) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            watcher.start_watching().await?;
            
            // Start the event processing loop
            if let Some(mut receiver) = self.event_receiver.take() {
                let aggregator = self.event_aggregator.clone();
                let clients = self.connected_clients.clone();
                
                tokio::spawn(async move {
                    while let Some(event) = receiver.recv().await {
                        log::info!("File change detected: {:?}", event);
                        
                        // Add to aggregator
                        {
                            let mut agg = aggregator.write().await;
                            agg.add_event(event.clone());
                        }
                        
                        // Notify connected clients
                        Self::notify_clients(&clients, &event).await;
                    }
                });
            }
        }
        Ok(())
    }
    
    async fn notify_clients(
        clients: &Arc<RwLock<Vec<String>>>, 
        event: &FileChangeEvent
    ) {
        let client_list = clients.read().await;
        if !client_list.is_empty() {
            log::info!("Notifying {} MCP clients of file change: {}", 
                client_list.len(), 
                event.file_path.display()
            );
            
            // TODO: Implement actual MCP notification protocol
            // This would send MCP notifications to connected clients
            // For now, just log the notification
            for client_id in client_list.iter() {
                log::debug!("Would notify client {}: {:?}", client_id, event);
            }
        }
    }
    
    pub async fn add_client(&self, client_id: String) {
        let mut clients = self.connected_clients.write().await;
        if !clients.contains(&client_id) {
            clients.push(client_id);
        }
    }
    
    pub async fn remove_client(&self, client_id: &str) {
        let mut clients = self.connected_clients.write().await;
        clients.retain(|id| id != client_id);
    }
    
    pub async fn get_change_summary(&self) -> crate::watcher::ChangeEventSummary {
        let aggregator = self.event_aggregator.read().await;
        aggregator.get_summary()
    }
}