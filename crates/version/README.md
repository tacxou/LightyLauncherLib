# lighty-version

Version management and builders for [LightyLauncher](https://crates.io/crates/lighty-launcher).

## Overview

**Version**: 0.8.6
**Part of**: [LightyLauncher](https://crates.io/crates/lighty-launcher)

`lighty-version` provides version builders that implement the `VersionInfo` trait from `lighty-loaders`, enabling version management for Minecraft instances.

## Features

- **VersionBuilder** - Standard version builder for all loaders
- **LightyVersionBuilder** - Custom server version builder for LightyUpdater
- **VersionInfo Implementation** - Implements core version trait
- **Directory Management** - Configurable game and Java directories
- **Custom Content Injection** - Add custom mods, libraries, and assets
- **Automatic Merge via LoaderExtensionsWithMerge** - Seamless integration with installer
- **Type Safety** - Strongly typed version information

## Quick Start

```toml
[dependencies]
lighty-version = "0.8.6"
```

## Adding Custom Mods, Libraries, and Assets

### Overview

With `LoaderExtensionsWithMerge`, you can add custom mods, libraries, and assets to any loader instance. These are automatically merged into the loader's metadata during installation.

### Builder Methods

```rust
let instance = VersionBuilder::new(...)
    .with_mods(vec![
        Mods {
            name: "my-mod.jar".to_string(),
            url: Some("https://...".to_string()),
            path: Some("mods/my-mod.jar".to_string()),
            sha1: Some("hash...".to_string()),
            size: Some(1024000),
        }
    ])
    .with_libraries(vec![
        Library {
            name: "com.example:lib:1.0".to_string(),
            url: Some("https://...".to_string()),
            path: Some("libraries/com/example/lib/1.0/lib-1.0.jar".to_string()),
            sha1: Some("hash...".to_string()),
            size: Some(2048000),
        }
    ])
    .with_assets(AssetsFile { objects: HashMap::new() });
```

### Automatic Merge During Installation

The `LoaderExtensionsWithMerge` trait automatically merges custom content when the installer runs:

```rust
use lighty_version::LoaderExtensionsWithMerge;

// Custom content automatically merged when metadata is fetched
let metadata = instance.get_metadata_merged().await?;

// Or use directly in launch - the merge happens automatically
instance.launch(&profile, JavaDistribution::Temurin)
    .run()
    .await?;
```

### VersionBuilder (Standard Loaders)

```rust
use lighty_core::AppState;
use lighty_version::VersionBuilder;
use lighty_loaders::types::Loader;

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "MyLauncher";
const APPLICATION: &str = "";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _app = AppState::new(
        QUALIFIER.to_string(),
        ORGANIZATION.to_string(),
        APPLICATION.to_string(),
    )?;

    let launcher_dir = AppState::get_project_dirs();

    // Create instance with Fabric loader
    let instance = VersionBuilder::new(
        "my-instance",         // Instance name
        Loader::Fabric,        // Loader type
        "0.16.9",             // Loader version
        "1.21.1",             // Minecraft version
        launcher_dir
    );

    println!("Instance: {}", instance.name());
    println!("Game dir: {}", instance.game_dirs().display());

    Ok(())
}
```

### LightyVersionBuilder (Custom Server)

```rust
use lighty_core::AppState;
use lighty_version::LightyVersionBuilder;

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "MyLauncher";
const APPLICATION: &str = "";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _app = AppState::new(
        QUALIFIER.to_string(),
        ORGANIZATION.to_string(),
        APPLICATION.to_string(),
    )?;

    let launcher_dir = AppState::get_project_dirs();

    // Create instance for custom server with LightyUpdater
    let instance = LightyVersionBuilder::new(
        "my-modpack",                  // Instance name
        "https://myserver.com/api",    // Server API URL
        launcher_dir
    );

    println!("Server: {}", instance.loader_version());

    Ok(())
}
```

### With Custom Directories

```rust
use std::path::PathBuf;

// Custom game and Java directories
let instance = VersionBuilder::new(
    "custom",
    Loader::Vanilla,
    "",
    "1.21.1",
    launcher_dir
)
.with_custom_game_dir(PathBuf::from("/opt/minecraft/instances/custom"))
.with_custom_java_dir(PathBuf::from("/usr/lib/jvm/java-21"));
```

## Core Types

| Type | Description |
|------|-------------|
| **VersionBuilder** | Standard version builder for all loaders |
| **LightyVersionBuilder** | Custom server builder for LightyUpdater |

Both implement `VersionInfo` from `lighty-loaders`.

## Documentation

📚 **[Complete Documentation](./docs)**

| Guide | Description |
|-------|-------------|
| [How to Use](./docs/how-to-use.md) | Practical usage guide with examples |
| [Overview](./docs/overview.md) | Architecture and design |
| [Exports](./docs/exports.md) | Complete export reference |
| [VersionBuilder](./docs/version-builder.md) | Standard version builder details |
| [LightyVersionBuilder](./docs/lighty-version-builder.md) | Custom server builder details |

## Related Crates

- **[lighty-launcher](../../../README.md)** - Main package
- **[lighty-loaders](../loaders/README.md)** - VersionInfo trait and loaders
- **[lighty-core](../core/README.md)** - AppState for project directories
- **[lighty-launch](../launch/README.md)** - Uses VersionBuilder for launching

## License

MIT
