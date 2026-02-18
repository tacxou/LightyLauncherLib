// Copyright (c) 2025 Hamadi
// Licensed under the MIT License

//! Library installation module

use lighty_loaders::types::{VersionInfo, version_metadata::Library};
use lighty_core::time_it;
use crate::errors::InstallerResult;
use super::verifier::{needs_download, verify_files_in_allowlist};
use super::downloader::download_with_concurrency_limit;
use std::collections::HashSet;

#[cfg(feature = "events")]
use lighty_event::EventBus;

/// Collects libraries that need to be downloaded
pub async fn collect_library_tasks(
    version: &impl VersionInfo,
    libraries: &[Library],
) -> Vec<(String, std::path::PathBuf)> {
    let parent_path = version.game_dirs().join("libraries");
    let mut tasks = Vec::new();

    for lib in libraries {
        let Some(url) = &lib.url else { continue };
        let Some(path_str) = &lib.path else { continue };

        let path = parent_path.join(path_str);

        if needs_download(&path, lib.sha1.as_ref(), &lib.name).await {
            tasks.push((url.clone(), path));
        }
    }

    tasks
}

/// Collects libraries that need to be downloaded with allowlist verification
///
/// This function verifies that all libraries are in the allowed list before processing.
/// Libraries outside the allowlist will be blocked from installation.
///
/// # Arguments
/// * `version` - Game version info
/// * `libraries` - Slice of libraries to process
/// * `allowed_files` - Set of allowed file paths (can use patterns like "*.jar", "libraries/*")
///
/// # Returns
/// A vector of tuples containing (url, path) for libraries that need to be downloaded
pub async fn collect_library_tasks_with_allowlist(
    version: &impl VersionInfo,
    libraries: &[Library],
    allowed_files: &HashSet<String>,
) -> InstallerResult<Vec<(String, std::path::PathBuf)>> {
    // Extract file paths from libraries for verification
    let lib_paths: Vec<&str> = libraries
        .iter()
        .filter_map(|l| l.path.as_ref().map(|p| p.as_str()))
        .collect();

    // Verify all library paths are authorized
    verify_files_in_allowlist(&lib_paths, allowed_files)?;

    let parent_path = version.game_dirs().join("libraries");
    let mut tasks = Vec::new();

    for lib in libraries {
        let Some(url) = &lib.url else { continue };
        let Some(path_str) = &lib.path else { continue };

        let path = parent_path.join(path_str);

        if needs_download(&path, lib.sha1.as_ref(), &lib.name).await {
            tasks.push((url.clone(), path));
        }
    }

    Ok(tasks)
}

/// Downloads libraries from pre-collected tasks
pub async fn download_libraries(
    tasks: Vec<(String, std::path::PathBuf)>,
    #[cfg(feature = "events")] event_bus: Option<&EventBus>,
) -> InstallerResult<()> {
    if tasks.is_empty() {
        lighty_core::trace_info!("[Installer] ✓ All libraries already cached and verified");
        return Ok(());
    }

    lighty_core::trace_info!("[Installer] Downloading {} libraries...", tasks.len());
    time_it!("Libraries download", {
        download_with_concurrency_limit(
            tasks,
            #[cfg(feature = "events")]
            event_bus,
        )
        .await?
    });
    lighty_core::trace_info!("[Installer] ✓ Libraries installed");
    Ok(())
}
