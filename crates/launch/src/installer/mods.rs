// Copyright (c) 2025 Hamadi
// Licensed under the MIT License

//! Mod installation module

use lighty_loaders::types::{VersionInfo, version_metadata::Mods};
use lighty_core::time_it;
use crate::errors::InstallerResult;
use super::verifier::{needs_download, verify_mods_in_whitelist, verify_files_in_allowlist};
use super::downloader::download_with_concurrency_limit;
use std::collections::HashSet;

#[cfg(feature = "events")]
use lighty_event::EventBus;

/// Collects mods that need to be downloaded
pub async fn collect_mod_tasks(
    version: &impl VersionInfo,
    mods: &[Mods],
) -> Vec<(String, std::path::PathBuf)> {
    // Don't create mods directory if there are no mods
    if mods.is_empty() {
        return Vec::new();
    }

    // Remove .join("runtime") since mods are typically placed directly in the "mods" directory, not in a "runtime/mods" subdirectory.
    let parent_path = version.game_dirs().join("mods");

    // Create mods directory only if there are mods to install
    lighty_core::mkdir!(&parent_path);

    let mut tasks = Vec::new();

    for _mod in mods {
        let Some(url) = &_mod.url else { continue };
        let Some(path_str) = &_mod.path else { continue };

        let path = parent_path.join(path_str);

        if needs_download(&path, _mod.sha1.as_ref(), &_mod.name).await {
            tasks.push((url.clone(), path));
        }
    }

    tasks
}

/// Collects mods that need to be downloaded with whitelist verification
///
/// This function verifies that all mods are in the allowed list before processing.
/// This prevents unauthorized mods from being installed.
///
/// # Arguments
/// * `version` - Game version info
/// * `mods` - Slice of mods to process
/// * `allowed_mods` - Set of allowed mod names
///
/// # Returns
/// A vector of tuples containing (url, path) for mods that need to be downloaded
pub async fn collect_mod_tasks_verified(
    version: &impl VersionInfo,
    mods: &[Mods],
    allowed_mods: &HashSet<String>,
) -> InstallerResult<Vec<(String, std::path::PathBuf)>> {
    // Don't create mods directory if there are no mods
    if mods.is_empty() {
        return Ok(Vec::new());
    }

    // Verify all mods are authorized
    verify_mods_in_whitelist(mods, allowed_mods)?;

    // Remove .join("runtime") since mods are typically placed directly in the "mods" directory, not in a "runtime/mods" subdirectory.
    let parent_path = version.game_dirs().join("mods");

    // Create mods directory only if there are mods to install
    lighty_core::mkdir!(&parent_path);

    let mut tasks = Vec::new();

    for _mod in mods {
        let Some(url) = &_mod.url else { continue };
        let Some(path_str) = &_mod.path else { continue };

        let path = parent_path.join(path_str);

        if needs_download(&path, _mod.sha1.as_ref(), &_mod.name).await {
            tasks.push((url.clone(), path));
        }
    }

    Ok(tasks)
}

/// Collects mods that need to be downloaded with allowlist verification
///
/// This function verifies that all mods are in the allowed list before processing.
/// Files outside the allowlist will be blocked from installation.
///
/// # Arguments
/// * `version` - Game version info
/// * `mods` - Slice of mods to process
/// * `allowed_files` - Set of allowed file paths (can use patterns like "*.jar", "mods/*")
///
/// # Returns
/// A vector of tuples containing (url, path) for mods that need to be downloaded
pub async fn collect_mod_tasks_with_allowlist(
    version: &impl VersionInfo,
    mods: &[Mods],
    allowed_files: &HashSet<String>,
) -> InstallerResult<Vec<(String, std::path::PathBuf)>> {
    // Don't create mods directory if there are no mods
    if mods.is_empty() {
        return Ok(Vec::new());
    }

    // Extract file paths from mods for verification
    let mod_paths: Vec<&str> = mods
        .iter()
        .filter_map(|m| m.path.as_ref().map(|p| p.as_str()))
        .collect();

    // Verify all mod paths are authorized
    verify_files_in_allowlist(&mod_paths, allowed_files)?;

    // Remove .join("runtime") since mods are typically placed directly in the "mods" directory, not in a "runtime/mods" subdirectory.
    let parent_path = version.game_dirs().join("mods");

    // Create mods directory only if there are mods to install
    lighty_core::mkdir!(&parent_path);

    let mut tasks = Vec::new();

    for _mod in mods {
        let Some(url) = &_mod.url else { continue };
        let Some(path_str) = &_mod.path else { continue };

        let path = parent_path.join(path_str);

        if needs_download(&path, _mod.sha1.as_ref(), &_mod.name).await {
            tasks.push((url.clone(), path));
        }
    }

    Ok(tasks)
}

/// Downloads mods from pre-collected tasks
pub async fn download_mods(
    tasks: Vec<(String, std::path::PathBuf)>,
    #[cfg(feature = "events")] event_bus: Option<&EventBus>,
) -> InstallerResult<()> {
    if tasks.is_empty() {
        lighty_core::trace_info!("[Installer] ✓ All mods already cached and verified");
        return Ok(());
    }

    lighty_core::trace_info!("[Installer] Downloading {} mods...", tasks.len());
    time_it!("Mods download", {
        download_with_concurrency_limit(
            tasks,
            #[cfg(feature = "events")]
            event_bus,
        )
        .await?
    });
    lighty_core::trace_info!("[Installer] ✓ Mods installed");
    Ok(())
}
