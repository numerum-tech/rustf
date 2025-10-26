use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use anyhow::Result;
use std::fs;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInstance {
    pub name: Option<String>,
    pub port: u16,
    pub websocket_port: Option<u16>,
    pub bind_address: String,
    pub project_path: PathBuf,
    pub pid: u32,
    pub started_at: DateTime<Utc>,
    pub watch_enabled: bool,
    pub websocket_enabled: bool,
}

impl ServerInstance {
    pub fn new(
        name: Option<String>,
        port: u16,
        bind_address: String,
        project_path: PathBuf,
        watch_enabled: bool,
        websocket_enabled: bool,
    ) -> Self {
        let websocket_port = if websocket_enabled {
            Some(port + 1)
        } else {
            None
        };

        Self {
            name,
            port,
            websocket_port,
            bind_address,
            project_path,
            pid: std::process::id(),
            started_at: Utc::now(),
            watch_enabled,
            websocket_enabled,
        }
    }

    pub fn is_alive(&self) -> bool {
        // Check if process is still running
        #[cfg(unix)]
        {
            use std::process::Command;
            
            match Command::new("kill")
                .arg("-0")
                .arg(self.pid.to_string())
                .output() 
            {
                Ok(output) => output.status.success(),
                Err(_) => false,
            }
        }
        
        #[cfg(windows)]
        {
            // On Windows, try to open the process
            use std::ptr;
            use winapi::um::processthreadsapi::OpenProcess;
            use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;
            use winapi::um::handleapi::CloseHandle;
            
            unsafe {
                let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, self.pid);
                if handle != ptr::null_mut() {
                    CloseHandle(handle);
                    true
                } else {
                    false
                }
            }
        }
        
        #[cfg(not(any(unix, windows)))]
        {
            // Fallback: assume it's alive
            true
        }
    }
}

pub struct InstanceRegistry {
    instances: Arc<RwLock<HashMap<u16, ServerInstance>>>,
    registry_file: PathBuf,
}

impl InstanceRegistry {
    pub fn new() -> Result<Self> {
        let registry_file = Self::get_registry_file_path()?;
        let instances = Self::load_instances(&registry_file)?;
        
        Ok(Self {
            instances: Arc::new(RwLock::new(instances)),
            registry_file,
        })
    }

    fn get_registry_file_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        
        let rustf_dir = home_dir.join(".rustf-cli");
        fs::create_dir_all(&rustf_dir)?;
        
        Ok(rustf_dir.join("instances.json"))
    }

    fn load_instances(registry_file: &PathBuf) -> Result<HashMap<u16, ServerInstance>> {
        if !registry_file.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(registry_file)?;
        let instances: HashMap<u16, ServerInstance> = serde_json::from_str(&content)
            .unwrap_or_else(|_| HashMap::new());

        // Filter out dead instances
        let alive_instances: HashMap<u16, ServerInstance> = instances
            .into_iter()
            .filter(|(_, instance)| instance.is_alive())
            .collect();

        Ok(alive_instances)
    }

    async fn save_instances(&self) -> Result<()> {
        let instances = self.instances.read().await;
        let json = serde_json::to_string_pretty(&*instances)?;
        
        let mut file = fs::File::create(&self.registry_file)?;
        file.write_all(json.as_bytes())?;
        
        Ok(())
    }

    pub async fn register(&self, instance: ServerInstance) -> Result<()> {
        let mut instances = self.instances.write().await;
        instances.insert(instance.port, instance);
        drop(instances);
        
        self.save_instances().await?;
        Ok(())
    }

    pub async fn unregister(&self, port: u16) -> Result<()> {
        let mut instances = self.instances.write().await;
        instances.remove(&port);
        drop(instances);
        
        self.save_instances().await?;
        Ok(())
    }

    pub async fn get_instance(&self, port: u16) -> Option<ServerInstance> {
        let instances = self.instances.read().await;
        instances.get(&port).cloned()
    }

    pub async fn list_instances(&self) -> Vec<ServerInstance> {
        let mut instances = self.instances.write().await;
        
        // Remove dead instances
        let dead_ports: Vec<u16> = instances
            .iter()
            .filter(|(_, instance)| !instance.is_alive())
            .map(|(port, _)| *port)
            .collect();
        
        for port in dead_ports {
            instances.remove(&port);
        }
        
        let alive_instances: Vec<ServerInstance> = instances.values().cloned().collect();
        drop(instances);
        
        // Save the cleaned up list
        let _ = self.save_instances().await;
        
        alive_instances
    }

    pub async fn find_by_name(&self, name: &str) -> Option<ServerInstance> {
        let instances = self.instances.read().await;
        instances
            .values()
            .find(|instance| {
                instance.name.as_ref()
                    .map(|n| n == name)
                    .unwrap_or(false)
            })
            .cloned()
    }

    pub async fn is_port_in_use(&self, port: u16) -> bool {
        let instances = self.instances.read().await;
        if let Some(instance) = instances.get(&port) {
            instance.is_alive()
        } else {
            false
        }
    }

    pub async fn find_available_port(&self, start_port: u16, max_attempts: u16) -> Option<u16> {
        let instances = self.instances.read().await;
        
        for offset in 0..max_attempts {
            let port = start_port + offset;
            
            // Check if port is in registry
            if instances.contains_key(&port) {
                continue;
            }
            
            // Check if port is actually available
            if !Self::is_port_available(port) {
                continue;
            }
            
            // For WebSocket mode, also check port+1
            if instances.values().any(|i| i.websocket_port == Some(port)) {
                continue;
            }
            
            return Some(port);
        }
        
        None
    }

    fn is_port_available(port: u16) -> bool {
        use std::net::{TcpListener, SocketAddr};
        
        let addr: SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .unwrap_or_else(|_| ([127, 0, 0, 1], port).into());
        
        TcpListener::bind(addr).is_ok()
    }
}

// Global instance registry
lazy_static::lazy_static! {
    static ref GLOBAL_REGISTRY: InstanceRegistry = {
        InstanceRegistry::new()
            .expect("Failed to create instance registry")
    };
}

pub fn get_registry() -> &'static InstanceRegistry {
    &GLOBAL_REGISTRY
}