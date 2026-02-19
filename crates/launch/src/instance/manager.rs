use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::SystemTime;
// use tokio::process::Child;

use super::errors::{InstanceError, InstanceResult};

/// Internal representation of a running game instance
#[allow(dead_code)]
pub(crate) struct GameInstance {
    /// Process ID
    pub pid: u32,
    /// Instance name
    pub instance_name: String,
    /// Version string (e.g., "1.20.1-fabric-0.15.0")
    pub version: String,
    /// Username used to launch
    pub username: String,
    /// Game directory path
    pub game_dir: PathBuf,
    /// Launch timestamp
    pub started_at: SystemTime,
}

/// Internal manager for tracking running game instances
pub(crate) struct InstanceManager {
    instances: RwLock<HashMap<u32, GameInstance>>,
}

/// Global instance manager
pub(crate) static INSTANCE_MANAGER: Lazy<InstanceManager> = Lazy::new(InstanceManager::new);

impl InstanceManager {
    /// Create a new instance manager
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
        }
    }

    /// Get the first PID for a given instance name
    pub fn get_pid(&self, instance_name: &str) -> Option<u32> {
        let instances = self.instances.read().unwrap();
        instances
            .values()
            .find(|inst| inst.instance_name == instance_name)
            .map(|inst| inst.pid)
    }

    /// Get all PIDs for a given instance name
    pub fn get_pids(&self, instance_name: &str) -> Vec<u32> {
        let instances = self.instances.read().unwrap();
        instances
            .values()
            .filter(|inst| inst.instance_name == instance_name)
            .map(|inst| inst.pid)
            .collect()
    }

    /// Register a new running instance
    pub async fn register_instance(&self, instance: GameInstance) {
        let mut instances = self.instances.write().unwrap();
        instances.insert(instance.pid, instance);
    }

    /// Unregister an instance by PID
    pub async fn unregister_instance(&self, pid: u32) {
        let mut instances = self.instances.write().unwrap();
        instances.remove(&pid);
    }

    /// Close an instance by PID
    ///
    /// Kills the process using the system's kill mechanism.
    /// The instance will be unregistered automatically by the console handler.
    pub async fn close_instance(&self, pid: u32) -> InstanceResult<()> {
        let mut instances = self.instances.write().unwrap();
        let _instance = instances
            .remove(&pid)
            .ok_or(InstanceError::NotFound { pid })?;

        drop(instances); // Release the lock

        // Kill the process using system kill
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            let output = Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .output()?;

            if !output.status.success() {
                lighty_core::trace_warn!(pid = pid, "Failed to kill process");
            } else {
                lighty_core::trace_info!(pid = pid, "Instance killed");
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            match kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                Ok(_) => {
                    lighty_core::trace_info!(pid = pid, "Instance killed");
                }
                Err(e) => {
                    lighty_core::trace_warn!(pid = pid, error = %e, "Failed to kill process");
                    return Err(InstanceError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to kill process: {}", e),
                    )));
                }
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    /// Check if there are any running instances
    pub fn has_running_instances(&self) -> bool {
        let instances = self.instances.read().unwrap();
        !instances.is_empty()
    }
}
