// Copyright (c) 2025 Hamadi
// Licensed under the MIT License

pub mod installer;
pub mod config;
mod downloader;
mod verifier;
mod libraries;
mod mods;
mod natives;
mod client;
mod assets;

// Re-export the Installer trait
pub use installer::Installer;
pub use installer::install_with_cleanup;

// Re-export file verification utilities
pub use verifier::{
    verify_mods_in_whitelist,
    create_mods_whitelist,
    verify_files_in_allowlist,
    create_files_allowlist,
    cleanup_unauthorized_files,
    cleanup_game_directories,
};
pub use mods::collect_mod_tasks_verified;
pub use mods::collect_mod_tasks_with_allowlist;
pub use libraries::collect_library_tasks_with_allowlist;
pub use natives::collect_native_tasks_with_allowlist;
