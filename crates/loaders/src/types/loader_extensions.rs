use crate::types::version_metadata::VersionMetaData;
use crate::types::{Loader, VersionInfo};
use crate::utils::error::QueryError;
#[allow(unused_imports)]
use crate::loaders::lighty_updater::lighty_updater::{LIGHTY_UPDATER, LightyQuery};
#[allow(unused_imports)]
use crate::loaders::neoforge::neoforge::{NeoForgeQuery, NEOFORGE};
#[allow(unused_imports)]
use crate::loaders::quilt::quilt::{QuiltQuery, QUILT};
#[allow(unused_imports)]
use crate::loaders::fabric::fabric::{FabricQuery, FABRIC};
#[allow(unused_imports)]
use crate::loaders::vanilla::vanilla::{VanillaQuery, VANILLA};
use async_trait::async_trait;
use std::sync::Arc;

pub type Result<T> = std::result::Result<T, QueryError>;

/// Extension trait for loading metadata from any loader
///
/// This trait provides a generic interface for fetching metadata from different mod loaders.
/// The main method is `get_metadata()`, which automatically dispatches to the correct
/// loader implementation based on `self.loader()`.
///
/// Specialized query methods are also available for retrieving specific parts of the metadata.
#[async_trait]
pub trait LoaderExtensions {
    /// Get complete metadata for the current loader
    ///
    /// This is the main method that should be used. It automatically dispatches
    /// to the appropriate repository based on the loader type.
    ///
    /// # Example
    /// ```no_run
    /// use lighty_loaders::{VersionBuilder, Loader, LoaderExtensions};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let instance = VersionBuilder::new("instance", Loader::Fabric, "0.16.9", "1.21.1", path);
    /// let metadata = instance.get_metadata().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn get_metadata(&self) -> Result<Arc<VersionMetaData>>;

    /// Get only libraries metadata
    ///
    /// This method fetches just the libraries information, which can be faster
    /// than fetching the complete metadata if you only need the libraries.
    async fn get_libraries(&self) -> Result<Arc<VersionMetaData>>;

    /// Get main class information (Vanilla-based loaders only)
    async fn get_main_class(&self) -> Result<Arc<VersionMetaData>>;

    /// Get native libraries (Vanilla-based loaders only)
    async fn get_natives(&self) -> Result<Arc<VersionMetaData>>;

    /// Get Java version requirement (Vanilla-based loaders only)
    async fn get_java_version(&self) -> Result<Arc<VersionMetaData>>;

    /// Get assets information (Vanilla-based loaders only)
    async fn get_assets(&self) -> Result<Arc<VersionMetaData>>;
}

/// Default implementation for any type that implements VersionInfo<LoaderType = Loader>
#[async_trait]
impl<T> LoaderExtensions for T
where
    T: VersionInfo<LoaderType = Loader> + Send + Sync,
{
    /// Get complete metadata by dispatching to the appropriate repository
    async fn get_metadata(&self) -> Result<Arc<VersionMetaData>> {
        match self.loader() {
            #[cfg(feature = "vanilla")]
            Loader::Vanilla => {
                VANILLA.get(self, VanillaQuery::VanillaBuilder).await
            }

            #[cfg(feature = "fabric")]
            Loader::Fabric => {
                FABRIC.get(self, FabricQuery::FabricBuilder).await
            }

            #[cfg(feature = "quilt")]
            Loader::Quilt => {
                QUILT.get(self, QuiltQuery::QuiltBuilder).await
            }

            #[cfg(feature = "neoforge")]
            Loader::NeoForge => {
                NEOFORGE.get(self, NeoForgeQuery::NeoForgeBuilder).await
            }

            #[cfg(feature = "forge")]
            Loader::Forge => {
                // TODO: Implement Forge support
                Err(QueryError::UnsupportedLoader(
                    "Forge loader is not yet implemented".to_string()
                ))
            }

            #[cfg(feature = "lighty_updater")]
            Loader::LightyUpdater => {
                LIGHTY_UPDATER.get(self, LightyQuery::LightyBuilder).await
            }

            // Fallback for unsupported loaders or disabled features
            _ => {
                Err(QueryError::UnsupportedLoader(
                    format!("Loader {:?} is not supported or feature is not enabled", self.loader())
                ))
            }
        }
    }

    /// Get libraries metadata for the current loader
    async fn get_libraries(&self) -> Result<Arc<VersionMetaData>> {
        match self.loader() {
            #[cfg(feature = "vanilla")]
            Loader::Vanilla => {
                VANILLA.get(self, VanillaQuery::Libraries).await
            }

            #[cfg(feature = "fabric")]
            Loader::Fabric => {
                FABRIC.get(self, FabricQuery::Libraries).await
            }

            #[cfg(feature = "quilt")]
            Loader::Quilt => {
                QUILT.get(self, QuiltQuery::Libraries).await
            }

            #[cfg(feature = "neoforge")]
            Loader::NeoForge => {
                // NeoForge doesn't have a separate libraries query, use full metadata
                NEOFORGE.get(self, NeoForgeQuery::NeoForgeBuilder).await
            }

            #[cfg(feature = "forge")]
            Loader::Forge => {
                // TODO: Implement Forge support
                Err(QueryError::UnsupportedLoader(
                    "Forge loader is not yet implemented".to_string()
                ))
            }

            _ => {
                Err(QueryError::UnsupportedLoader(
                    format!("get_libraries() not supported for {:?}", self.loader())
                ))
            }
        }
    }

    /// Get main class (Vanilla-based queries only)
    async fn get_main_class(&self) -> Result<Arc<VersionMetaData>> {
        #[cfg(feature = "vanilla")]
        {
            VANILLA.get(self, VanillaQuery::MainClass).await
        }

        #[cfg(not(feature = "vanilla"))]
        {
            Err(QueryError::UnsupportedLoader(
                "get_main_class() requires vanilla feature".to_string()
            ))
        }
    }

    /// Get natives (Vanilla-based queries only)
    async fn get_natives(&self) -> Result<Arc<VersionMetaData>> {
        #[cfg(feature = "vanilla")]
        {
            VANILLA.get(self, VanillaQuery::Natives).await
        }

        #[cfg(not(feature = "vanilla"))]
        {
            Err(QueryError::UnsupportedLoader(
                "get_natives() requires vanilla feature".to_string()
            ))
        }
    }

    /// Get Java version requirement (Vanilla-based queries only)
    async fn get_java_version(&self) -> Result<Arc<VersionMetaData>> {
        #[cfg(feature = "vanilla")]
        {
            VANILLA.get(self, VanillaQuery::JavaVersion).await
        }

        #[cfg(not(feature = "vanilla"))]
        {
            Err(QueryError::UnsupportedLoader(
                "get_java_version() requires vanilla feature".to_string()
            ))
        }
    }

    /// Get assets (Vanilla-based queries only)
    async fn get_assets(&self) -> Result<Arc<VersionMetaData>> {
        #[cfg(feature = "vanilla")]
        {
            VANILLA.get(self, VanillaQuery::Assets).await
        }

        #[cfg(not(feature = "vanilla"))]
        {
            Err(QueryError::UnsupportedLoader(
                "get_assets() requires vanilla feature".to_string()
            ))
        }
    }
}
