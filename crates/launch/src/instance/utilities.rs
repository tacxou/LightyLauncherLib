use lighty_loaders::types::{InstanceSize, VersionInfo};
use lighty_loaders::types::version_metadata::Version;

use super::errors::{InstanceError, InstanceResult};
use super::INSTANCE_MANAGER;

/// Extension trait providing instance management utilities
///
/// This trait is automatically implemented for any type that implements `VersionInfo`.
/// It provides methods to manage running game instances and calculate instance sizes.
///
/// # Usage
///
/// To use these utilities, you must import the trait:
///
/// ```rust,ignore
/// use lighty_launch::InstanceControl;
/// use lighty_version::VersionBuilder;
/// use lighty_loaders::types::Loader;
///
/// let minozia = VersionBuilder::new(
///     "minozia",
///     Loader::Fabric,
///     "0.15.0",
///     "1.20.1",
///     &launcher_dir,
/// );
///
/// // Now you can use instance control methods
/// if let Some(pid) = minozia.get_pid() {
///     println!("Instance is running with PID: {}", pid);
///     minozia.close_instance(pid).await?;
/// }
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// use lighty_launch::InstanceControl;
///
/// // Get the first PID
/// if let Some(pid) = instance.get_pid() {
///     println!("PID: {}", pid);
/// }
///
/// // Get all PIDs (if multiple instances are running)
/// let pids = instance.get_pids();
/// for pid in pids {
///     println!("Running PID: {}", pid);
/// }
///
/// // Close a specific instance
/// instance.close_instance(pid).await?;
///
/// // Delete the instance (only if not running)
/// instance.delete_instance().await?;
///
/// // Calculate instance size
/// let version = instance.get_metadata().await?;
/// let size = instance.size_of_instance(&version);
/// println!("Total size: {}", size.total_human());
/// ```
pub trait InstanceControl: VersionInfo {
    /// Get the first PID of a running instance
    ///
    /// Returns `Some(pid)` if the instance is running, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if let Some(pid) = instance.get_pid() {
    ///     println!("Instance is running with PID: {}", pid);
    /// } else {
    ///     println!("Instance is not running");
    /// }
    /// ```
    fn get_pid(&self) -> Option<u32> {
        INSTANCE_MANAGER.get_pid(self.name())
    }

    /// Get all PIDs if the instance is running multiple times
    ///
    /// Returns a vector of all PIDs associated with this instance name.
    /// Returns an empty vector if no instances are running.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let pids = instance.get_pids();
    /// if pids.is_empty() {
    ///     println!("No instances running");
    /// } else {
    ///     println!("Running instances: {:?}", pids);
    /// }
    /// ```
    fn get_pids(&self) -> Vec<u32> {
        INSTANCE_MANAGER.get_pids(self.name())
    }

    /// Close a specific instance by PID
    ///
    /// Attempts a graceful shutdown with a 5-second timeout.
    /// If the instance doesn't respond, it will be force-killed.
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID of the instance to close
    ///
    /// # Errors
    ///
    /// Returns `InstanceError::NotFound` if no instance with the given PID exists.
    /// Returns `InstanceError::Io` if there's an I/O error during shutdown.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if let Some(pid) = instance.get_pid() {
    ///     instance.close_instance(pid).await?;
    ///     println!("Instance closed");
    /// }
    /// ```
    fn close_instance(
        &self,
        pid: u32,
    ) -> impl std::future::Future<Output = InstanceResult<()>> + '_ {
        async move { INSTANCE_MANAGER.close_instance(pid).await }
    }

    /// Delete the instance completely from disk
    ///
    /// This removes all instance files, including saves, configs, mods, etc.
    ///
    /// # Safety
    ///
    /// The instance must not be running. If any instances are running,
    /// this method will return an error without deleting anything.
    ///
    /// # Errors
    ///
    /// Returns `InstanceError::StillRunning` if any instances are running.
    /// Returns `InstanceError::Io` if there's an I/O error during deletion.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Make sure no instances are running
    /// let pids = instance.get_pids();
    /// for pid in pids {
    ///     instance.close_instance(pid).await?;
    /// }
    ///
    /// // Now safe to delete
    /// instance.delete_instance().await?;
    /// println!("Instance deleted");
    /// ```
    fn delete_instance(&self) -> impl std::future::Future<Output = InstanceResult<()>> + '_ {
        async move {
            // Check that no instances are running
            let running_pids = self.get_pids();
            if !running_pids.is_empty() {
                return Err(InstanceError::StillRunning {
                    instance_name: self.name().to_string(),
                    pids: running_pids,
                });
            }

            // Complete deletion
            tokio::fs::remove_dir_all(self.game_dirs()).await?;

            // Emit event
            #[cfg(feature = "events")]
            {
                use lighty_event::{Event, InstanceDeletedEvent, EVENT_BUS};
                use std::time::SystemTime;

                EVENT_BUS.emit(Event::InstanceDeleted(InstanceDeletedEvent {
                    instance_name: self.name().to_string(),
                    timestamp: SystemTime::now(),
                }));
            }

            lighty_core::trace_info!(instance = %self.name(), "Instance deleted");
            Ok(())
        }
    }

    /// Calculate instance size from Version metadata
    ///
    /// This calculates the total disk space that will be used by the instance,
    /// broken down by component (libraries, mods, natives, client, assets).
    ///
    /// # Arguments
    ///
    /// * `version` - The version metadata containing size information
    ///
    /// # Returns
    ///
    /// An `InstanceSize` struct with detailed size breakdown and helper methods.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let version = instance.get_metadata().await?;
    /// let size = instance.size_of_instance(&version);
    ///
    /// // Formatted sizes
    /// println!("Libraries: {}", InstanceSize::format(size.libraries));
    /// println!("Mods: {}", InstanceSize::format(size.mods));
    /// println!("Natives: {}", InstanceSize::format(size.natives));
    /// println!("Client: {}", InstanceSize::format(size.client));
    /// println!("Assets: {}", InstanceSize::format(size.assets));
    /// println!("Total: {}", InstanceSize::format(size.total));
    ///
    /// // Or MB/GB values
    /// println!("Total MB: {:.2}", size.total_mb());
    /// println!("Total GB: {:.2}", size.total_gb());
    /// ```
    fn size_of_instance(&self, version: &Version) -> InstanceSize {
        let libraries_size = version.libraries.iter().filter_map(|lib| lib.size).sum();

        let mods_size = version
            .mods
            .as_ref()
            .map(|mods| mods.iter().filter_map(|m| m.size).sum())
            .unwrap_or(0);

        let natives_size = version
            .natives
            .as_ref()
            .map(|natives| natives.iter().filter_map(|n| n.size).sum())
            .unwrap_or(0);

        let client_size = version.client.as_ref().and_then(|c| c.size).unwrap_or(0);

        let assets_size = version
            .assets_index
            .as_ref()
            .and_then(|idx| idx.total_size)
            .unwrap_or(0);

        let total = libraries_size + mods_size + natives_size + client_size + assets_size;

        InstanceSize {
            libraries: libraries_size,
            mods: mods_size,
            natives: natives_size,
            client: client_size,
            assets: assets_size,
            total,
        }
    }
}

// Automatic implementation for any type that implements VersionInfo
impl<T: VersionInfo> InstanceControl for T {}
