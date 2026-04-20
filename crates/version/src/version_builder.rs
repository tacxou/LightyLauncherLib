use std::{fmt::Debug,
          path::{Path, PathBuf},
};
use once_cell::sync::Lazy;
use directories::ProjectDirs;
use lighty_loaders::types::VersionInfo;
use lighty_loaders::types::version_metadata::{Library, Mods, AssetsFile};

/// Structure principale pour configurer une version Minecraft avec un loader
///
/// # Exemples
///
/// ```rust
/// use lighty_version::VersionBuilder;
/// use lighty_loaders::types::Loader;
///
/// let builder = VersionBuilder::new("my-profile", Loader::Vanilla, "0.15.0", "1.20.1", &PROJECT_DIRS);
///
/// // Ou avec le builder pattern:
/// let builder = VersionBuilder::new("my-profile", Loader::Vanilla, "0.15.0", "1.20.1", &PROJECT_DIRS)
///     .with_custom_game_dir(PathBuf::from("./games"))
///     .with_custom_java_dir(PathBuf::from("./java"));
/// ```
#[derive(Debug, Clone)]
pub struct VersionBuilder<'a, L = ()> {
    pub name: String,
    pub loader: L,
    pub loader_version: String,
    pub minecraft_version: String,
    pub project_dirs: &'a Lazy<ProjectDirs>,
    pub game_dirs: PathBuf,
    pub java_dirs: PathBuf,
    pub custom_mods: Option<Vec<Mods>>,
    pub custom_files: Option<Vec<Mods>>,
    pub custom_assets: Option<AssetsFile>,
    pub custom_libraries: Option<Vec<Library>>,
}

impl<'a, L> VersionBuilder<'a, L> {
    /// Crée un nouveau VersionBuilder avec les paramètres par défaut
    ///
    /// Les répertoires par défaut sont :
    /// - `game_dirs`: `{data_dir}/{name}`
    /// - `java_dirs`: `{config_dir}/jre`
    pub fn new(
        name: &str,
        loader: L,
        loader_version: &str,
        minecraft_version: &str,
        project_dirs: &'a Lazy<ProjectDirs>,
    ) -> Self {
        Self {
            name: name.to_string(),
            loader,
            loader_version: loader_version.to_string(),
            minecraft_version: minecraft_version.to_string(),
            project_dirs,
            game_dirs: project_dirs.data_dir().join(name),
            java_dirs: project_dirs.config_dir().to_path_buf().join("jre"),
            custom_mods: None,
            custom_files: None,
            custom_assets: None,
            custom_libraries: None,
        }
    }

    /// Définit un répertoire de jeu personnalisé
    ///
    /// # Exemple
    /// ```rust
    /// let builder = VersionBuilder::new(...)
    ///     .with_custom_game_dir(PathBuf::from("./custom/games"));
    /// ```
    pub fn with_custom_game_dir(mut self, game_dir: PathBuf) -> Self {
        self.game_dirs = game_dir;
        self
    }

    /// Définit un répertoire Java personnalisé
    ///
    /// # Exemple
    /// ```rust
    /// let builder = VersionBuilder::new(...)
    ///     .with_custom_java_dir(PathBuf::from("./custom/java"));
    /// ```
    pub fn with_custom_java_dir(mut self, java_dir: PathBuf) -> Self {
        self.java_dirs = java_dir;
        self
    }

    /// Change le loader
    pub fn with_loader(mut self, loader: L) -> Self {
        self.loader = loader;
        self
    }

    /// Change la version du loader
    pub fn with_loader_version(mut self, version: &str) -> Self {
        self.loader_version = version.to_string();
        self
    }

    /// Change la version de Minecraft
    pub fn with_minecraft_version(mut self, version: &str) -> Self {
        self.minecraft_version = version.to_string();
        self
    }

    /// Ajoute des mods personnalisés
    ///
    /// # Exemple
    /// ```rust
    /// let builder = VersionBuilder::new(...)
    ///     .with_mods(vec![
    ///         Mods {
    ///             name: "MyMod-1.0.jar".to_string(),
    ///             url: Some("https://...".to_string()),
    ///             path: Some("mods/MyMod-1.0.jar".to_string()),
    ///             sha1: Some("...".to_string()),
    ///             size: Some(1024000),
    ///         }
    ///     ]);
    /// ```
    pub fn with_mods(mut self, mods: Vec<Mods>) -> Self {
        self.custom_mods = Some(mods);
        self
    }

    /// Ajoute des fichiers personnalisés
    ///
    /// Ajoute des fichiers personnalisés
    ///
    /// Ces fichiers sont fusionnés avec les mods custom lors de la construction finale.
    pub fn with_files(mut self, files: Vec<Mods>) -> Self {
        self.custom_files = Some(files);
        self
    }

    /// Ajoute des assets personnalisés
    ///
    /// # Exemple
    /// ```rust
    /// let builder = VersionBuilder::new(...)
    ///     .with_assets(AssetsFile { objects: HashMap::new() });
    /// ```
    pub fn with_assets(mut self, assets: AssetsFile) -> Self {
        self.custom_assets = Some(assets);
        self
    }

    /// Ajoute des libraries personnalisées
    ///
    /// # Exemple
    /// ```rust
    /// let builder = VersionBuilder::new(...)
    ///     .with_libraries(vec![
    ///         Library {
    ///             name: "com.example:lib:1.0".to_string(),
    ///             url: Some("https://...".to_string()),
    ///             path: Some("libraries/com/example/lib/1.0/lib-1.0.jar".to_string()),
    ///             sha1: Some("...".to_string()),
    ///             size: Some(2048000),
    ///         }
    ///     ]);
    /// ```
    pub fn with_libraries(mut self, libraries: Vec<Library>) -> Self {
        self.custom_libraries = Some(libraries);
        self
    }
}

impl<'a, L: Clone + Send + Sync + Debug> VersionInfo for VersionBuilder<'a, L> {
    type LoaderType = L;

    fn name(&self) -> &str {
        &self.name
    }

    fn loader_version(&self) -> &str {
        &self.loader_version
    }

    fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    fn game_dirs(&self) -> &Path {
        &self.game_dirs
    }

    fn java_dirs(&self) -> &Path {
        &self.java_dirs
    }

    fn loader(&self) -> &Self::LoaderType {
        &self.loader
    }

    fn get_custom_mods(&self) -> Option<&Vec<Mods>> {
        self.custom_mods.as_ref()
    }

    fn get_custom_files(&self) -> Option<&Vec<Mods>> {
        self.custom_files.as_ref()
    }
    
    fn get_custom_libraries(&self) -> Option<&Vec<Library>> {
        self.custom_libraries.as_ref()
    }
    
    fn get_custom_assets(&self) -> Option<&AssetsFile> {
        self.custom_assets.as_ref()
    }
}

// Implémentation pour &VersionBuilder (permet de passer des références)
impl<'a, 'b, L: Clone + Send + Sync + Debug> VersionInfo for &'b VersionBuilder<'a, L> {
    type LoaderType = L;

    fn name(&self) -> &str {
        &self.name
    }

    fn loader_version(&self) -> &str {
        &self.loader_version
    }

    fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    fn game_dirs(&self) -> &Path {
        &self.game_dirs
    }

    fn java_dirs(&self) -> &Path {
        &self.java_dirs
    }

    fn loader(&self) -> &Self::LoaderType {
        &self.loader
    }
}


