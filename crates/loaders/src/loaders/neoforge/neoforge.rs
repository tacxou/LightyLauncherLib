use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::{collections::HashMap, fs::File, io::Read, path::PathBuf};
use zip::ZipArchive;

use super::neoforge_metadata::{NeoForgeMetaData, NeoForgeVersionMeta};
use super::patcher;
use crate::types::version_metadata::{Arguments, Library, MainClass, Version, VersionMetaData};
use crate::types::VersionInfo;
use crate::utils::{error::QueryError, manifest::ManifestRepository, query::Query};

use crate::loaders::vanilla::vanilla::VanillaQuery;

use lighty_core::download::download_file_untracked;
use lighty_core::hosts::HTTP_CLIENT;

use lighty_core::mkdir;
pub type Result<T> = std::result::Result<T, QueryError>;

pub static NEOFORGE: Lazy<ManifestRepository<NeoForgeQuery>> = Lazy::new(|| ManifestRepository::new());

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NeoForgeQuery {
    Libraries,
    Arguments,
    MainClass,
    NeoForgeBuilder,
}

#[async_trait]
impl Query for NeoForgeQuery {
    type Query = NeoForgeQuery;
    type Data = VersionMetaData;
    type Raw = NeoForgeMetaData ;

    fn name() -> &'static str {
        "neoforge"
    }

    async fn fetch_full_data<V: VersionInfo>(version: &V) -> Result<NeoForgeMetaData> {
        // Construire l'URL de l'installer
        let installer_url = build_installer_url(version);

        lighty_core::trace_debug!(url = %installer_url, loader = "neoforge", "Installer URL constructed");

        let profiles_dir = version.game_dirs().join(".neoforge");
        mkdir!(profiles_dir);

        let installer_path = profiles_dir.join(format!("neoforge-{}-installer.jar", version.loader_version()));

        // Vérifier et télécharger l'installer si nécessaire
        let needs_download = if installer_path.exists() {
            match verify_installer_sha1(&installer_path, &installer_url).await {
                Ok(true) => {
                    lighty_core::trace_info!(loader = "neoforge", "Installer already exists and SHA1 is valid");
                    false
                }
                Ok(false) => {
                    lighty_core::trace_warn!(loader = "neoforge", "Installer exists but SHA1 mismatch, re-downloading");
                    true
                }
                Err(_e) => {
                    lighty_core::trace_warn!(error = %_e, loader = "neoforge", "Could not verify SHA1, using existing file");
                    false
                }
            }
        } else {
            true
        };

        if needs_download {
            lighty_core::trace_info!(path = ?installer_path, loader = "neoforge", "Downloading installer");
            download_file_untracked(&installer_url, &installer_path)
                .await
                .map_err(|e| QueryError::Conversion {
                    message: format!("Failed to download installer: {}", e)
                })?;

            if let Ok(valid) = verify_installer_sha1(&installer_path, &installer_url).await {
                if !valid {
                    return Err(QueryError::Conversion {
                        message: "Downloaded installer has invalid SHA1".to_string()
                    });
                }
            }
        }

        // Lire les JSONs directement depuis le JAR
        let (install_profile, _) = read_jsons_from_jar(&installer_path).await?;

        lighty_core::trace_info!(loader = "neoforge", "Successfully loaded NeoForge metadata");

        Ok(install_profile)
    }

    async fn extract<V: VersionInfo>(version: &V, query: &Self::Query, full_data: &NeoForgeMetaData) -> Result<Self::Data> {
        let result = match query {
            NeoForgeQuery::Libraries => VersionMetaData::Libraries(extract_libraries(full_data)),
            &NeoForgeQuery::Arguments | &NeoForgeQuery::MainClass => todo!(),
            NeoForgeQuery::NeoForgeBuilder => {
                VersionMetaData::Version(Self::version_builder(version, full_data).await?)
            }
        };
        Ok(result)
    }

    async fn version_builder<V: VersionInfo>(version: &V, _full_data: &NeoForgeMetaData) -> Result<Version> {
        // Paralléliser la récupération des données Vanilla et la lecture du version.json
        let (vanilla_builder, version_meta) = tokio::try_join!(
            async {
                let vanilla_data = VanillaQuery::fetch_full_data(version).await?;
                VanillaQuery::version_builder(version, &vanilla_data).await
            },
            async {
                let profiles_dir = version.game_dirs().join(".neoforge");
                let installer_path = profiles_dir.join(format!("neoforge-{}-installer.jar", version.loader_version()));
                let (_, version_meta) = read_jsons_from_jar(&installer_path).await?;
                Ok::<_, QueryError>(version_meta)
            }
        )?;

        // Extraire UNIQUEMENT les bibliothèques du version.json (runtime)
        // Les bibliothèques de l'install_profile.json sont uniquement pour les processors
        let version_json_libs = extract_libraries_from_version_meta(&version_meta);

        // Fusionner : Vanilla + version.json (priorité au version.json)
        let merged_libs = merge_libraries(vanilla_builder.libraries, version_json_libs);

        // Merger directement avec Vanilla en priorité
        Ok(Version {
            main_class: merge_main_class(vanilla_builder.main_class, extract_main_class(&version_meta)),
            java_version: vanilla_builder.java_version,
            arguments: merge_arguments(vanilla_builder.arguments, extract_arguments(&version_meta)),
            libraries: merged_libs,
            mods: None,
            natives: vanilla_builder.natives,
            client: vanilla_builder.client,
            assets_index: vanilla_builder.assets_index,
            assets: vanilla_builder.assets,
        })
    }
}

/// --------- Fonctions de merge ----------
fn merge_main_class(vanilla: MainClass, neoforge: MainClass) -> MainClass {
    if neoforge.main_class.is_empty() {
        vanilla
    } else {
        neoforge
    }
}

fn merge_arguments(vanilla: Arguments, neoforge: Arguments) -> Arguments {
    Arguments {
        game: {
            let mut args = vanilla.game;
            args.extend(neoforge.game);
            args
        },
        jvm: match (vanilla.jvm, neoforge.jvm) {
            (Some(mut v), Some(n)) => {
                v.extend(n);
                Some(v)
            }
            (Some(v), None) => Some(v),
            (None, Some(n)) => Some(n),
            (None, None) => None,
        },
    }
}

/// Évite les doublons en comparant group:artifact (sans version)
fn merge_libraries(vanilla_libs: Vec<Library>, neoforge_libs: Vec<Library>) -> Vec<Library> {
    let capacity = vanilla_libs.len() + neoforge_libs.len();
    let mut lib_map: HashMap<String, Library> = HashMap::with_capacity(capacity);

    // Ajouter Vanilla d'abord
    for lib in vanilla_libs {
        let key = extract_artifact_key(&lib.name);
        lib_map.insert(key, lib);
    }

    // NeoForge écrase Vanilla si même artifact (version plus récente)
    for lib in neoforge_libs {
        let key = extract_artifact_key(&lib.name);
        lib_map.insert(key, lib);
    }

    lib_map.into_values().collect()
}

/// Extrait "group:artifact" (sans version) pour identifier les doublons
fn extract_artifact_key(maven_name: &str) -> String {
    let mut parts = maven_name.split(':');
    match (parts.next(), parts.next()) {
        (Some(group), Some(artifact)) => format!("{}:{}", group, artifact),
        _ => maven_name.to_string(),
    }
}

/// --------- Fonctions d'extraction ----------
fn extract_main_class(version_meta: &NeoForgeVersionMeta) -> MainClass {
    MainClass {
        main_class: version_meta.main_class.clone(),
    }
}

fn extract_arguments(version_meta: &NeoForgeVersionMeta) -> Arguments {
    Arguments {
        game: version_meta.arguments.game.clone(),
        jvm: Some(version_meta.arguments.jvm.clone()),
    }
}

fn extract_libraries(full_data: &NeoForgeMetaData) -> Vec<Library> {
    full_data
        .libraries
        .iter()
        .map(|lib| Library {
            name: lib.name.clone(),
            url: Some(lib.downloads.artifact.url.clone()),
            path: Some(lib.downloads.artifact.path.clone()),
            sha1: Some(lib.downloads.artifact.sha1.clone()),
            size: Some(lib.downloads.artifact.size),
        })
        .collect()
}

fn extract_libraries_from_version_meta(version_meta: &NeoForgeVersionMeta) -> Vec<Library> {
    version_meta
        .libraries
        .iter()
        .map(|lib| Library {
            name: lib.name.clone(),
            url: Some(lib.downloads.artifact.url.clone()),
            path: Some(lib.downloads.artifact.path.clone()),
            sha1: Some(lib.downloads.artifact.sha1.clone()),
            size: Some(lib.downloads.artifact.size),
        })
        .collect()
}

/// --------- Helpers ----------
fn is_old_neoforge<V: VersionInfo>(version: &V) -> bool {
    version_compare::compare_to(version.minecraft_version(), "1.20.1", version_compare::Cmp::Le)
        .unwrap_or(false)
}

fn build_installer_url<V: VersionInfo>(version: &V) -> String {
    if is_old_neoforge(version) {
        let path_version = format!("{}-{}", version.minecraft_version(), version.loader_version());
        let file_prefix = format!("forge-{}", version.minecraft_version());
        format!(
            "https://maven.neoforged.net/releases/net/neoforged/forge/{}/{}-{}-installer.jar",
            path_version, file_prefix, version.loader_version()
        )
    } else {
        format!(
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
            version.loader_version(), version.loader_version()
        )
    }
}

fn processors_marker_path<V: VersionInfo>(version: &V) -> PathBuf {
    version
        .game_dirs()
        .join(".neoforge")
        .join(format!(
            "processors-{}-{}.sha1",
            version.minecraft_version(),
            version.loader_version()
        ))
}

/// Lit les JSONs directement depuis le JAR sans extraction sur disque
async fn read_jsons_from_jar(installer_path: &PathBuf) -> Result<(NeoForgeMetaData, NeoForgeVersionMeta)> {
    let path = installer_path.clone();

    tokio::task::spawn_blocking(move || {
        let file = File::open(&path).map_err(|e| QueryError::Conversion {
            message: format!("Failed to open installer JAR: {}", e)
        })?;

        let mut archive = ZipArchive::new(file).map_err(|e| QueryError::Conversion {
            message: format!("Failed to open ZIP archive: {}", e)
        })?;

        // Lire install_profile.json
        let install_profile = {
            let mut file = archive.by_name("install_profile.json").map_err(|_| {
                QueryError::MissingField {
                    field: "install_profile.json in installer JAR".to_string(),
                }
            })?;

            let mut contents = String::new();
            file.read_to_string(&mut contents).map_err(|e| QueryError::Conversion {
                message: format!("Failed to read install_profile.json: {}", e)
            })?;

            serde_json::from_str::<NeoForgeMetaData>(&contents)?
        };

        // Lire version.json
        let version_meta = {
            let mut file = archive.by_name("version.json").map_err(|_| {
                QueryError::MissingField {
                    field: "version.json in installer JAR".to_string(),
                }
            })?;

            let mut contents = String::new();
            file.read_to_string(&mut contents).map_err(|e| QueryError::Conversion {
                message: format!("Failed to read version.json: {}", e)
            })?;

            serde_json::from_str::<NeoForgeVersionMeta>(&contents)?
        };

        Ok((install_profile, version_meta))
    })
    .await
    .map_err(|e| QueryError::Conversion {
        message: format!("Failed to spawn blocking task: {}", e)
    })?
}

/// Récupère le SHA1 attendu depuis Maven
async fn fetch_maven_sha1(jar_url: &str) -> Option<String> {
    let sha1_url = format!("{}.sha1", jar_url);

    match HTTP_CLIENT.get(&sha1_url).send().await {
        Ok(response) if response.status().is_success() => {
            response.text().await.ok().and_then(|text| {
                let sha1 = text.trim().split_whitespace().next()?.to_string();
                (sha1.len() == 40).then_some(sha1)
            })
        }
        _ => None,
    }
}

/// Vérifie le SHA1 de l'installer
async fn verify_installer_sha1(installer_path: &PathBuf, installer_url: &str) -> Result<bool> {
    let expected_sha1 = fetch_maven_sha1(installer_url)
        .await
        .ok_or_else(|| QueryError::Conversion {
            message: "Failed to fetch SHA1 from Maven".to_string()
        })?;

    lighty_core::verify_file_sha1_sync(installer_path, &expected_sha1)
        .map_err(|e| QueryError::Conversion {
            message: format!("Failed to verify SHA1: {}", e)
        })
}

/// Exécute les processors pour une installation NeoForge
/// Cette fonction doit être appelée après que les libraries sont téléchargées
pub async fn run_install_processors<V: VersionInfo>(
    version: &V,
    install_profile: &NeoForgeMetaData,
) -> Result<()> {
    lighty_core::trace_info!(loader = "neoforge", "Checking if processors need to run");

    let profiles_dir = version.game_dirs().join(".neoforge");
    mkdir!(profiles_dir);
    let installer_path = profiles_dir.join(format!(
        "neoforge-{}-installer.jar",
        version.loader_version(),
    ));

    if !installer_path.exists() {
        return Err(QueryError::Conversion {
            message: "Installer JAR not found. Run fetch_full_data first.".to_string(),
        });
    }

    let installer_url = build_installer_url(version);
    let marker_path = processors_marker_path(version);
    if let Some(expected_sha1) = fetch_maven_sha1(&installer_url).await {
        if let Ok(existing) = std::fs::read_to_string(&marker_path) {
            if existing.trim() == expected_sha1 {
                lighty_core::trace_info!(
                    loader = "neoforge",
                    "Processors already executed for this installer, skipping"
                );
                return Ok(());
            }
        }
    }

    // Exécuter les processors
    patcher::run_processors(version, install_profile, installer_path).await?;
    
    if let Some(expected_sha1) = fetch_maven_sha1(&installer_url).await {
        if let Err(_err) = std::fs::write(&marker_path, expected_sha1) {
            lighty_core::trace_warn!(
                error = %_err,
                loader = "neoforge",
                "Failed to write processors marker file"
            );
        }
    }

    lighty_core::trace_info!(loader = "neoforge", "Processors completed successfully");
    Ok(())
}