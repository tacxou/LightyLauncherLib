use lighty_java::JreError;
use lighty_core::time_it;
use lighty_java::jre_downloader::jre_download;
use lighty_java::JavaDistribution;
use lighty_loaders::types::version_metadata::Version;
use crate::errors::{InstallerError, InstallerResult};
use crate::installer::Installer;
use super::builder::LaunchBuilder;
use lighty_loaders::types::{Loader, LoaderExtensions, VersionInfo};
use lighty_auth::UserProfile;
use std::sync::Arc;
use std::path::PathBuf;
use lighty_loaders::types::version_metadata::VersionMetaData;
use lighty_java::jre_downloader::find_java_binary;
use lighty_java::runtime::JavaRuntime;
use crate::arguments::Arguments;
use std::collections::{HashMap,HashSet};

#[cfg(feature = "neoforge")]
use lighty_loaders::neoforge::neoforge::{run_install_processors, NEOFORGE};

#[cfg(feature = "events")]
use lighty_event::EventBus;

pub trait Launch {
    /// Launch the game with a builder pattern
    ///
    /// # Arguments
    /// - `profile`: User profile from authentication
    /// - `java_distribution`: Java distribution to use
    ///
    /// # Returns
    /// A `LaunchBuilder` for configuring JVM options and game arguments
    ///
    /// # Example
    /// ```no_run
    /// // Simple launch
    /// version.launch(&profile, JavaDistribution::Zulu).await?;
    ///
    /// // With custom options
    /// version.launch(&profile, JavaDistribution::Zulu)
    ///     .with_jvm_options()
    ///         .set("Xmx", "4G")
    ///         .done()
    ///     .with_arguments()
    ///         .set(KEY_WIDTH, "1920")
    ///         .done()
    ///     .await?;
    /// ```
    fn launch<'a>(&'a mut self, profile: &'a UserProfile, java_distribution: JavaDistribution) -> LaunchBuilder<'a, Self>
    where
        Self: Sized;
}

// Implémentation générique pour tout type implémentant VersionInfo + les traits nécessaires
impl<T> Launch for T
where
    T: VersionInfo<LoaderType = Loader> + LoaderExtensions + Arguments + Installer,
{
    fn launch<'a>(&'a mut self, profile: &'a UserProfile, java_distribution: JavaDistribution) -> LaunchBuilder<'a, Self> {
        LaunchBuilder::new(self, profile, java_distribution)
    }
}

/// Internal function to execute the launch process
pub(crate) async fn execute_launch<T>(
    version: &mut T,
    profile: &UserProfile,
    java_distribution: JavaDistribution,
    jvm_overrides: &std::collections::HashMap<String, String>,
    jvm_removals: &std::collections::HashSet<String>,
    arg_overrides: &std::collections::HashMap<String, String>,
    arg_removals: &std::collections::HashSet<String>,
    raw_args: &[String],
    #[cfg(feature = "events")] event_bus: Option<&EventBus>,
) -> InstallerResult<()>
where
    T: VersionInfo<LoaderType = Loader> + LoaderExtensions + Arguments + Installer,
{
        let username = &profile.username;
        let uuid = &profile.uuid;
        // 1. Préparer les métadonnées du loader
        let metadata = prepare_metadata(
            version,
            #[cfg(feature = "events")]
            event_bus,
        ).await?;

        let version_data = extract_version(&metadata)?;

        // 2. S'assurer que Java est installé
        let java_path = ensure_java_installed(
            version,
            version_data,
            &java_distribution,
            #[cfg(feature = "events")]
            event_bus,
        ).await?;

        // 3. Installer les dépendances Minecraft
        time_it!("Install delay", version.install(
            version_data,
            #[cfg(feature = "events")]
            event_bus,
        ).await?);

        // 3b. Exécuter les processors NeoForge (après download des libraries)
        // TODO: Ajouter un système de processors pour les loaders qui en ont besoin (ex: NeoForge) 
        // (à exécuter après l'installation des libraries mais avant le lancement du jeu)
        #[cfg(feature = "neoforge")]
        if matches!(version.loader(), Loader::NeoForge) {
            let install_profile = NEOFORGE.get_raw(version).await?;
            run_install_processors(version, install_profile.as_ref()).await?;
        }

        // 4. Lancer le jeu
        execute_game(
            version,
            version_data,
            username,
            uuid,
            java_path,
            arg_overrides,
            arg_removals,
            jvm_overrides,
            jvm_removals,
            raw_args,
            #[cfg(feature = "events")]
            event_bus,
        ).await
}

/// Récupère les métadonnées complètes du loader
async fn prepare_metadata<T>(
    builder: &mut T,
    #[cfg(feature = "events")] event_bus: Option<&EventBus>,
) -> InstallerResult<Arc<VersionMetaData>>
where
    T: VersionInfo<LoaderType = Loader> + LoaderExtensions,
{
    lighty_core::trace_debug!("[Launch] Fetching metadata for loader: {:?}", builder.loader());

    let loader_name = format!("{:?}", builder.loader());

    #[cfg(feature = "events")]
    if let Some(bus) = event_bus {
        bus.emit(lighty_event::Event::Loader(lighty_event::LoaderEvent::FetchingData {
            loader: loader_name.clone(),
            minecraft_version: builder.minecraft_version().to_string(),
            loader_version: builder.loader_version().to_string(),
        }));
    }

    // Generic metadata fetching - automatically dispatches to the correct loader
    let metadata = builder.get_metadata().await?;

    #[cfg(feature = "events")]
    if let Some(bus) = event_bus {
        bus.emit(lighty_event::Event::Loader(lighty_event::LoaderEvent::DataFetched {
            loader: loader_name,
            minecraft_version: builder.minecraft_version().to_string(),
            loader_version: builder.loader_version().to_string(),
        }));
    }

    lighty_core::trace_info!("[Launch] Metadata fetched successfully for {:?}", builder.loader());
    Ok(metadata)
}

/// S'assure que Java est installé et retourne le chemin vers l'exécutable
async fn ensure_java_installed<T>(
    builder: &T,
    version: &Version,
    java_distribution: &JavaDistribution,
    #[cfg(feature = "events")] event_bus: Option<&EventBus>,
) -> InstallerResult<PathBuf>
where
    T: VersionInfo,
{
    let java_version = version.java_version.major_version;

    // Vérifier si Java est déjà installé
    match find_java_binary(builder.java_dirs(), java_distribution, &java_version).await {
        Ok(path) => {
            lighty_core::trace_info!("[Java] Java {} already installed at: {:?}", java_version, path);

            #[cfg(feature = "events")]
            if let Some(bus) = event_bus {
                bus.emit(lighty_event::Event::Java(lighty_event::JavaEvent::JavaAlreadyInstalled {
                    distribution: java_distribution.get_name().to_string(),
                    version: java_version,
                    binary_path: path.to_string_lossy().to_string(),
                }));
            }

            Ok(path)
        }
        Err(_) => {
            lighty_core::trace_info!("[Java] Java {} not found, downloading...", java_version);

            #[cfg(feature = "events")]
            if let Some(bus) = event_bus {
                bus.emit(lighty_event::Event::Java(lighty_event::JavaEvent::JavaNotFound {
                    distribution: java_distribution.get_name().to_string(),
                    version: java_version,
                }));
            }

            #[cfg(feature = "events")]
            let path = jre_download(
                builder.java_dirs(),
                java_distribution,
                &java_version,
                |current, total| {
                    lighty_core::trace_debug!("[Java] Download progress: {}/{}", current, total);
                },
                event_bus,
            ).await.map_err(|e| InstallerError::DownloadFailed(format!("JRE download failed: {}", e)))?;

            #[cfg(not(feature = "events"))]
            let path = jre_download(
                builder.java_dirs(),
                java_distribution,
                &java_version,
                |current, total| {
                    lighty_core::trace_debug!("[Java] Download progress: {}/{}", current, total);
                },
            ).await.map_err(|e : JreError | InstallerError::DownloadFailed(format!("JRE download failed: {}", e)))?;

            lighty_core::trace_info!("[Java] Java {} installed successfully", java_version);
            Ok(path)
        }
    }
}

/// Lance le jeu avec les arguments appropriés
async fn execute_game<T>(
    builder: &T,
    version: &Version,
    username: &str,
    uuid: &str,
    java_path: PathBuf,
    arg_overrides: &HashMap<String, String>,
    arg_removals: &HashSet<String>,
    jvm_overrides: &HashMap<String, String>,
    jvm_removals: &HashSet<String>,
    raw_args: &[String],
    #[cfg(feature = "events")] event_bus: Option<&EventBus>,
) -> InstallerResult<()>
where
    T: VersionInfo + Arguments,
{
    use crate::instance::{handle_console_streams, INSTANCE_MANAGER};
    use crate::instance::manager::GameInstance;

    // Construire les arguments
    let arguments = builder.build_arguments(version, username, uuid, arg_overrides, arg_removals, jvm_overrides, jvm_removals, raw_args);

    // Créer JavaRuntime avec le chemin vers java.exe
    let java_runtime = JavaRuntime::new(java_path);
    lighty_core::trace_info!("[Launch] Executing game...");

    match java_runtime.execute(arguments, builder.game_dirs()).await {
        Ok(child) => {
            let pid = child.id().ok_or(InstallerError::NoPid)?;

            lighty_core::trace_info!("[Launch] Game launched successfully, PID: {}", pid);

            // Enregistrer l'instance (metadata only, no child handle)
            let instance = GameInstance {
                pid,
                instance_name: builder.name().to_string(),
                version: format!("{}-{}", builder.minecraft_version(), builder.loader_version()),
                username: username.to_string(),
                game_dir: builder.game_dirs().to_path_buf(),
                started_at: std::time::SystemTime::now(),
            };

            INSTANCE_MANAGER.register_instance(instance).await;

            // Émettre événement InstanceLaunched
            #[cfg(feature = "events")]
            if let Some(bus) = event_bus {
                use lighty_event::{Event, InstanceLaunchedEvent};

                bus.emit(Event::InstanceLaunched(InstanceLaunchedEvent {
                    pid,
                    instance_name: builder.name().to_string(),
                    version: format!("{}-{}", builder.minecraft_version(), builder.loader_version()),
                    username: username.to_string(),
                    timestamp: std::time::SystemTime::now(),
                }));

                // Spawner le détecteur de fenêtre
                let bus_clone = bus.clone();
                let instance_name = builder.name().to_string();
                let version = format!("{}-{}", builder.minecraft_version(), builder.loader_version());
                tokio::spawn(detect_window_appearance(pid, instance_name, version, bus_clone));
            }

            // Spawner le console streaming handler
            // This takes ownership of the child and handles all I/O until exit
            tokio::spawn(handle_console_streams(
                pid,
                builder.name().to_string(),
                child,
                #[cfg(feature = "events")]
                event_bus.cloned(),
            ));

            Ok(())
        }
        Err(e) => {
            lighty_core::trace_error!("[Launch] Failed to launch game: {}", e);
            Err(InstallerError::DownloadFailed(format!("Launch failed: {}", e)))
        }
    }
}

/// Extrait l'objet Version depuis VersionMetaData
fn extract_version(metadata: &VersionMetaData) -> InstallerResult<&Version> {
    match metadata {
        VersionMetaData::Version(v) => Ok(v),
        _ => Err(InstallerError::InvalidMetadata),
    }
}

/// Détecte l'apparition de la fenêtre du jeu et émet un événement
#[cfg(feature = "events")]
async fn detect_window_appearance(
    pid: u32,
    instance_name: String,
    version: String,
    event_bus: lighty_event::EventBus,
) {
    #[cfg(windows)]
    {
        use std::time::Duration;

        // Vérifier toutes les 100ms pendant 30 secondes maximum
        let max_attempts = 300;
        let check_interval = Duration::from_millis(100);

        for _ in 0..max_attempts {
            if has_visible_window(pid) {
                lighty_core::trace_info!("[Launch] Window appeared for PID: {}", pid);

                event_bus.emit(lighty_event::Event::InstanceWindowAppeared(
                    lighty_event::InstanceWindowAppearedEvent {
                        pid,
                        instance_name,
                        version,
                        timestamp: std::time::SystemTime::now(),
                    }
                ));
                return;
            }

            tokio::time::sleep(check_interval).await;
        }

        lighty_core::trace_warn!("[Launch] Window detection timed out for PID: {}", pid);
    }

    #[cfg(not(windows))]
    {
        // Sur les autres plateformes, attendre un délai fixe (approximation)
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        lighty_core::trace_info!("[Launch] Assuming window appeared for PID: {} (non-Windows platform)", pid);

        event_bus.emit(lighty_event::Event::InstanceWindowAppeared(
            lighty_event::InstanceWindowAppearedEvent {
                pid,
                instance_name,
                version,
                timestamp: std::time::SystemTime::now(),
            }
        ));
    }
}

/// Vérifie si un processus a une fenêtre visible (Windows uniquement)
#[cfg(all(windows, feature = "events"))]
fn has_visible_window(pid: u32) -> bool {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };

    struct EnumData {
        target_pid: u32,
        found: bool,
    }

    unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut EnumData);

        // Vérifier si la fenêtre est visible
        if IsWindowVisible(hwnd).as_bool() {
            let mut window_pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut window_pid));

            if window_pid == data.target_pid {
                data.found = true;
                return BOOL(0); // Arrêter l'énumération
            }
        }

        BOOL(1) // Continuer l'énumération
    }

    let mut data = EnumData {
        target_pid: pid,
        found: false,
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut data as *mut _ as isize),
        );
    }

    data.found
}