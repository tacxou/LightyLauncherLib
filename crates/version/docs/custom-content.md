# Custom Content Injection (Mods, Libraries, Assets)

## Overview

The `LoaderExtensionsWithMerge` trait enables seamless injection of custom mods, libraries, and assets into any Minecraft loader instance. This is similar to how `lighty_updater` handles metadata merging, but works for any loader (Vanilla, Fabric, NeoForge, etc.).

## Architecture

### How It Works

1. **Custom Content Storage**: The `VersionBuilder` stores custom mods, libraries, and assets in optional fields
2. **HasCustomContent Trait**: Provides a common interface to check if an instance has custom content
3. **LoaderExtensionsWithMerge**: Intercepts metadata fetching and applies merge automatically
4. **Automatic Merge**: When `get_metadata()` is called, the merge is applied transparently

### Data Flow

```
VersionBuilder with custom mods
        ↓
LoaderExtensionsWithMerge::get_metadata_merged()
        ↓
Fetch loader metadata (Fabric, NeoForge, etc.)
        ↓
Apply merge (combine loader + custom content)
        ↓
Return merged metadata
        ↓
Installer downloads everything (including custom mods)
```

## Usage

### Adding Custom Mods

```rust
use lighty_version::VersionBuilder;
use lighty_loaders::types::{Loader, Mods};

let mut instance = VersionBuilder::new(
    "my-modpack",
    Loader::NeoForge,
    "21.1.219",
    "1.21.1",
    launcher_dir,
)
.with_mods(vec![
    Mods {
        name: "sodium-neoforge-0.6.13.jar".to_string(),
        url: Some("https://cdn.modrinth.com/data/AANobbMI/.../sodium-neoforge-0.6.13.jar".to_string()),
        path: Some("mods/sodium-neoforge-0.6.13.jar".to_string()),
        sha1: Some("abc123def456...".to_string()),
        size: Some(2048000),
    },
    Mods {
        name: "lithium-neoforge-0.12.1.jar".to_string(),
        url: Some("https://cdn.modrinth.com/data/gvQqBUqZ/.../lithium-neoforge-0.12.1.jar".to_string()),
        path: Some("mods/lithium-neoforge-0.12.1.jar".to_string()),
        sha1: Some("def456abc123...".to_string()),
        size: Some(1536000),
    },
]);
```

### Adding Custom Libraries

```rust
use lighty_loaders::types::version_metadata::Library;

let mut instance = VersionBuilder::new(...)
    .with_libraries(vec![
        Library {
            name: "com.google.code.gson:gson:2.8.9".to_string(),
            url: Some("https://repo1.maven.org/.../gson-2.8.9.jar".to_string()),
            path: Some("libraries/com/google/code/gson/gson/2.8.9/gson-2.8.9.jar".to_string()),
            sha1: Some("hash...".to_string()),
            size: Some(256000),
        },
    ]);
```

### Adding Custom Assets

```rust
use lighty_loaders::types::version_metadata::AssetsFile;
use std::collections::HashMap;

let mut assets = AssetsFile {
    objects: HashMap::new(),
};

// Add custom asset
assets.objects.insert(
    "custom-texture".to_string(),
    AssetObject {
        hash: "hash123".to_string(),
        size: 65536,
    },
);

let mut instance = VersionBuilder::new(...)
    .with_assets(assets);
```

## Retrieving Merged Metadata

### Option 1: Explicit Merge (for inspection)

```rust
use lighty_version::VersionBuilderMergeExt;

// Get metadata with custom content already merged
let metadata = instance.get_metadata_merged().await?;
```

### Option 2: Automatic Merge During Launch

The merge happens automatically through `LoaderExtensionsWithMerge`:

```rust
// Launch directly - custom content is automatically included
instance
    .launch(&profile, JavaDistribution::Temurin)
    .run()
    .await?;
```

The installer will then download all merged content (both loader defaults + custom).

## Merge Functions

The merge functions are public and can be used independently:

```rust
use lighty_version::apply_custom_content_merge;
use lighty_loaders::types::version_metadata::Version;

// Manually apply merge
let merged = apply_custom_content_merge(&builder, version);
```

### Merge Behavior

- **Libraries**: Custom libraries are appended to loader libraries
- **Mods**: Custom mods are appended to loader mods (or created if none exist)
- **Assets**: Custom asset objects are merged into loader assets

## Example: Complete Modpack

```rust
use lighty_launcher::prelude::*;
use lighty_loaders::types::{Loader, Mods};
use lighty_version::{VersionBuilder, VersionBuilderMergeExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _app = AppState::new("fr".to_string(), ".LightyLauncher".to_string(), "".to_string())?;
    let launcher_dir = AppState::get_project_dirs();
    
    let mut neoforge = VersionBuilder::new(
        "my-modpack",
        Loader::NeoForge,
        "21.1.219",
        "1.21.1",
        launcher_dir,
    )
    .with_mods(vec![
        Mods {
            name: "sodium-neoforge-0.6.13.jar".to_string(),
            url: Some("https://...".to_string()),
            path: Some("mods/sodium-neoforge-0.6.13.jar".to_string()),
            sha1: Some("abc123".to_string()),
            size: Some(2048000),
        },
    ]);

    // Authenticate
    let mut auth = OfflineAuth::new("Player");
    let profile = auth.authenticate().await?;

    // Get merged metadata (custom mods included)
    let metadata = neoforge.get_metadata_merged().await?;
    println!("Mods to download: {:?}", metadata.mods);

    // Launch - installer downloads everything
    neoforge
        .launch(&profile, JavaDistribution::Temurin)
        .run()
        .await?;

    Ok(())
}
```

## Traits Reference

### HasCustomContent

Implemented by any type that can have custom content:

```rust
pub trait HasCustomContent: VersionInfo {
    fn get_custom_mods(&self) -> Option<&Vec<Mods>>;
    fn get_custom_libraries(&self) -> Option<&Vec<Library>>;
    fn get_custom_assets(&self) -> Option<&AssetsFile>;
}
```

### LoaderExtensionsWithMerge

Extends `LoaderExtensions` with merge support:

```rust
pub trait LoaderExtensionsWithMerge: LoaderExtensions + HasCustomContent {
    fn get_metadata_merged(&self) 
        -> Pin<Box<dyn Future<Output = Result<Arc<VersionMetaData>>> + Send + '_>>;
}
```

## Limitations & Notes

- Custom content is stored in memory
- Merges are resolved at fetch time (during launch/installation)
- Multiple calls to `get_metadata_merged()` will refetch from loader each time
- URL validation is NOT performed by VersionBuilder (ensure valid URLs when adding mods)

## See Also

- [lighty_updater merge implementation](../loaders/src/loaders/lighty_updater/merge_metadata.rs)
- [Launch architecture](../../launch/README.md)
