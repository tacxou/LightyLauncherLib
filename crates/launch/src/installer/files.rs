// Copyright (c) 2025 Hamadi
// Licensed under the MIT License

//! Custom files installation module

use lighty_core::time_it;
use lighty_loaders::types::{version_metadata::Mods, VersionInfo};
use std::collections::HashSet;

use crate::errors::InstallerResult;

use super::downloader::download_with_concurrency_limit;
use super::verifier::{needs_download, verify_files_in_allowlist};

#[cfg(feature = "events")]
use lighty_event::EventBus;

/// Collects custom files that need to be downloaded.
///
/// Unlike mods, custom files are resolved from the game root directory,
/// using their declared relative path.
pub async fn collect_file_tasks(
    version: &impl VersionInfo,
    files: &[Mods],
) -> Vec<(String, std::path::PathBuf)> {
    if files.is_empty() {
        return Vec::new();
    }

    let game_root = version.game_dirs().to_path_buf();
    let mut tasks = Vec::new();

    for file in files {
        let Some(url) = &file.url else { continue };
        let Some(path_str) = &file.path else { continue };

        let path = game_root.join(path_str);
        if needs_download(&path, file.sha1.as_ref(), &file.name).await {
            tasks.push((url.clone(), path));
        }
    }

    tasks
}

/// Collects custom files that need to be downloaded with allowlist verification.
pub async fn collect_file_tasks_with_allowlist(
    version: &impl VersionInfo,
    files: &[Mods],
    allowed_files: &HashSet<String>,
) -> InstallerResult<Vec<(String, std::path::PathBuf)>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let file_paths: Vec<&str> = files
        .iter()
        .filter_map(|f| f.path.as_ref().map(|p| p.as_str()))
        .collect();

    verify_files_in_allowlist(&file_paths, allowed_files)?;

    let game_root = version.game_dirs().to_path_buf();
    let mut tasks = Vec::new();

    for file in files {
        let Some(url) = &file.url else { continue };
        let Some(path_str) = &file.path else { continue };

        let path = game_root.join(path_str);
        if needs_download(&path, file.sha1.as_ref(), &file.name).await {
            tasks.push((url.clone(), path));
        }
    }

    Ok(tasks)
}

/// Downloads custom files from pre-collected tasks.
pub async fn download_files(
    tasks: Vec<(String, std::path::PathBuf)>,
    #[cfg(feature = "events")] event_bus: Option<&EventBus>,
) -> InstallerResult<()> {
    if tasks.is_empty() {
        lighty_core::trace_info!("[Installer] ✓ All custom files already cached and verified");
        return Ok(());
    }

    lighty_core::trace_info!("[Installer] Downloading {} custom files...", tasks.len());
    time_it!("Custom files download", {
        download_with_concurrency_limit(
            tasks,
            #[cfg(feature = "events")]
            event_bus,
        )
        .await?
    });
    lighty_core::trace_info!("[Installer] ✓ Custom files installed");
    Ok(())
}
