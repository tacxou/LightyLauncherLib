use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use lighty_core::download::download_file_untracked;
use lighty_core::mkdir;

use super::neoforge_metadata::{NeoForgeMetaData, Processor};
use crate::types::VersionInfo;
use crate::utils::error::QueryError;

pub type Result<T> = std::result::Result<T, QueryError>;

/// Contexte pour l'exécution d'un processor NeoForge
pub struct ProcessorContext {
    /// Répertoire de jeu
    pub game_dir: PathBuf,
    /// Répertoire des bibliothèques
    pub libraries_dir: PathBuf,
    /// Version Minecraft
    pub minecraft_version: String,
    /// Chemin vers le JAR de l'installer
    pub installer_path: PathBuf,
    /// Map des données avec substitutions
    pub data: HashMap<String, String>,
    /// Side (client ou server)
    pub side: String,
}

impl ProcessorContext {
    /// Crée un nouveau contexte pour l'exécution des processors
    pub fn new<V: VersionInfo>(
        version: &V,
        installer_path: PathBuf,
        metadata: &NeoForgeMetaData,
    ) -> Self {
        let side = "client".to_string();
        let game_dir = version.game_dirs();
        let libraries_dir = game_dir.join("libraries");

        // Construire la map de données pour les substitutions
        let mut data = HashMap::new();
        for (key, value) in &metadata.data {
            data.insert(key.clone(), value.client.clone());
        }

        // Ajouter les variables de base pour les substitutions
        data.insert("ROOT".to_string(), game_dir.to_string_lossy().to_string());
        data.insert(
            "LIBRARY_DIR".to_string(),
            libraries_dir.to_string_lossy().to_string(),
        );
        data.insert(
            "MINECRAFT_VERSION".to_string(),
            version.minecraft_version().to_string(),
        );

        // Ajouter le côté et le chemin vers l'installer
        data.insert("SIDE".to_string(), side.clone());
        data.insert(
            "INSTALLER".to_string(),
            installer_path.to_string_lossy().to_string(),
        );

        // Ajouter le chemin vers le JAR de Minecraft
        let minecraft_jar = game_dir.join(format!("{}.jar", version.name()));
        data.insert(
            "MINECRAFT_JAR".to_string(),
            minecraft_jar.to_string_lossy().to_string(),
        );

        Self {
            game_dir: game_dir.to_path_buf(),
            libraries_dir,
            minecraft_version: version.minecraft_version().to_string(),
            installer_path,
            data,
            side,
        }
    }

    /// Substitue uniquement les variables {VAR} sans résoudre les coordonnées Maven
    pub fn substitute_variables(&self, input: &str) -> String {
        let mut result = input.to_string();
        for (key, value) in &self.data {
            let pattern = format!("{{{}}}", key);
            result = result.replace(&pattern, value);
        }
        result
    }

    /// Substitue les patterns dans une chaîne de caractères
    ///
    /// Gère les variables {VAR}, les coordonnées Maven [group:artifact:version:classifier@extension] et les chemins d'installer /path
    pub fn substitute(&self, input: &str) -> Result<String> {
        let result = self.substitute_variables(input);

        // Gérer les coordonnées Maven [group:artifact:version:classifier@extension]
        if result.starts_with('[') && result.ends_with(']') {
            let maven_coords = &result[1..result.len() - 1];
            return self.resolve_maven_path(maven_coords);
        }

        // Gérer les chemins d'installer /path
        if result.starts_with('/') {
            return Ok(result);
        }

        Ok(result)
    }

    /// Résout les coordonnées Maven en un chemin de fichier
    /// 
    /// Supporte les formats :
    /// - group:artifact:version
    /// - group:artifact:version:classifier
    /// - group:artifact:version@extension
    /// - group:artifact:version:classifier@extension
    fn resolve_maven_path(&self, maven_coords: &str) -> Result<String> {
        let parts: Vec<&str> = maven_coords.split(':').collect();
        if parts.len() < 3 {
            return Err(QueryError::Conversion {
                message: format!("Invalid Maven coordinate: {}", maven_coords),
            });
        }

        let group = parts[0].replace('.', "/");
        let artifact = parts[1];

        // Gérer les différentes variantes de coordonnées Maven
        let (version, classifier, extension) = if parts.len() >= 4 {
            // Format: group:artifact:version:classifier[@extension]
            let version = parts[2];
            let last_part = parts[3];

            if let Some((clf, ext)) = last_part.split_once('@') {
                (version, Some(clf), ext)
            } else {
                (version, Some(last_part), "jar")
            }
        } else {
            // Format: group:artifact:version[@extension]
            let version_part = parts[2];

            if let Some((ver, ext)) = version_part.split_once('@') {
                (ver, None, ext)
            } else {
                (version_part, None, "jar")
            }
        };

        let filename = if let Some(clf) = classifier {
            format!("{}-{}-{}.{}", artifact, version, clf, extension)
        } else {
            format!("{}-{}.{}", artifact, version, extension)
        };

        let path = self
            .libraries_dir
            .join(&group)
            .join(artifact)
            .join(version)
            .join(&filename);

        Ok(path.to_string_lossy().to_string())
    }

    /// Extrait un fichier du JAR de l'installer vers un chemin de sortie
    pub async fn extract_installer_file(
        &self,
        internal_path: &str,
        output_path: &Path,
    ) -> Result<()> {
        // Supprimer le slash de début si présent
        let internal_path = internal_path.trim_start_matches('/');

        let file = File::open(&self.installer_path).map_err(|e| QueryError::Conversion {
            message: format!("Failed to open installer JAR: {}", e),
        })?;

        let mut archive = ZipArchive::new(file).map_err(|e| QueryError::Conversion {
            message: format!("Failed to open ZIP archive: {}", e),
        })?;

        let mut zip_file =
            archive
                .by_name(internal_path)
                .map_err(|_| QueryError::MissingField {
                    field: format!("{} in installer JAR", internal_path),
                })?;

        if let Some(parent) = output_path.parent() {
            mkdir!(parent);
        }

        let mut output = File::create(output_path).map_err(|e| QueryError::Conversion {
            message: format!("Failed to create output file: {}", e),
        })?;

        std::io::copy(&mut zip_file, &mut output).map_err(|e| QueryError::Conversion {
            message: format!("Failed to extract file: {}", e),
        })?;

        Ok(())
    }
}


/// Exécute tous les processors pour le côté spécifié
pub async fn run_processors<V: VersionInfo>(
    version: &V,
    metadata: &NeoForgeMetaData,
    installer_path: PathBuf,
) -> Result<()> {
    let context = ProcessorContext::new(version, installer_path, metadata);

    lighty_core::trace_info!(loader = "neoforge", "Starting processor execution");

    let total_processors = metadata.processors.len();

    // Filtrer les processors pour ne garder que ceux qui doivent s'exécuter côté client
    let processors: Vec<&Processor> = metadata
        .processors
        .iter()
        .filter(|p| {
            let should_execute = p.sides.is_empty() || p.sides.contains(&context.side);

            if !should_execute {
                lighty_core::trace_debug!(
                    loader = "neoforge",
                    jar = %p.jar,
                    sides = ?p.sides,
                    "Skipping processor (not for client side)"
                );
            }

            should_execute
        })
        .collect();

    let _skipped_count = total_processors - processors.len();

    lighty_core::trace_info!(
        loader = "neoforge",
        total = total_processors,
        client_processors = processors.len(),
        skipped = _skipped_count,
        "Filtered processors for client side"
    );

    for (_idx, processor) in processors.iter().enumerate() {
        lighty_core::trace_info!(
            loader = "neoforge",
            processor_num = _idx + 1,
            total = processors.len(),
            jar = %processor.jar,
            "Executing processor"
        );

        execute_processor(&context, processor).await?;
    }

    lighty_core::trace_info!(loader = "neoforge", "All processors completed successfully");

    Ok(())
}

/// Exécute un processor individuel
async fn execute_processor(context: &ProcessorContext, processor: &Processor) -> Result<()> {
    // 0. Vérifier les outputs pour voir si on peut sauter ce processor
    if should_skip_processor(context, processor)? {
        lighty_core::trace_info!(
            loader = "neoforge",
            "Processor outputs already exist, skipping"
        );
        return Ok(());
    }

    // 1. Télécharger le JAR du processor et les dépendances du classpath
    let jar_path = download_processor_jar(context, &processor.jar).await?;
    let mut classpath_paths = vec![jar_path.clone()];

    for cp in &processor.classpath {
        let cp_path = download_processor_jar(context, cp).await?;
        classpath_paths.push(cp_path);
    }

    // 2. Substituer les arguments du processor
    let mut processed_args = Vec::new();
    for arg in &processor.args {
        // D'abord substituer uniquement les variables {VAR}
        let substituted = context.substitute_variables(arg);

        if substituted.starts_with('[') && substituted.ends_with(']') {
            // Coordonnée Maven : résoudre le chemin et télécharger si le fichier n'existe pas
            let maven_coords = &substituted[1..substituted.len() - 1];
            let resolved_path = context.resolve_maven_path(maven_coords)?;
            let path = PathBuf::from(&resolved_path);
            if !path.exists() {
                // Tenter le téléchargement — si 404, c'est un output de processeur qui sera généré
                match download_processor_jar(context, maven_coords).await {
                    Ok(_) => {}
                    Err(_) => {
                        if let Some(parent) = path.parent() {
                            mkdir!(parent);
                        }
                        lighty_core::trace_debug!(
                            loader = "neoforge",
                            artifact = %maven_coords,
                            "Artifact not available on Maven, assuming processor output"
                        );
                    }
                }
            }
            processed_args.push(resolved_path);
        } else if substituted.starts_with('/') {
            // Chemin d'installer : extraire depuis le JAR
            let internal_path = &substituted[1..];
            let target_path = extract_and_resolve(context, internal_path).await?;
            processed_args.push(target_path);
        } else {
            processed_args.push(substituted);
        }
    }

    // 3. Extraire la Main-Class du JAR du processor
    let main_class = extract_main_class(&jar_path)?;

    // 4. Construire le classpath pour Java
    let classpath_strings: Vec<String> = classpath_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let classpath = classpath_strings.join(if cfg!(windows) { ";" } else { ":" });

    lighty_core::trace_debug!(
        loader = "neoforge",
        main_class = %main_class,
        classpath_count = classpath_paths.len(),
        args_count = processed_args.len(),
        "Processor configuration prepared"
    );

    // 5. Exécuter le processor avec Java
    // TODO: Intégrer avec le système Java de lighty_launcher
    let output = tokio::process::Command::new("java")
        .arg("-cp")
        .arg(&classpath)
        .arg(&main_class)
        .args(&processed_args)
        .current_dir(&context.game_dir)
        .output()
        .await
        .map_err(|e| QueryError::Conversion {
            message: format!("Failed to execute processor: {}", e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(QueryError::Conversion {
            message: format!(
                "Processor failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
                stdout, stderr
            ),
        });
    }

    lighty_core::trace_debug!(loader = "neoforge", "Processor completed successfully");

    Ok(())
}

/// Télécharge le JAR d'un processor à partir de ses coordonnées Maven et retourne le chemin local vers le fichier téléchargé
async fn download_processor_jar(context: &ProcessorContext, maven_coords: &str) -> Result<PathBuf> {
    let file_path = context.resolve_maven_path(maven_coords)?;
    let path = PathBuf::from(&file_path);

    if path.exists() {
        return Ok(path);
    }

    // Construire l'URL de téléchargement à partir des coordonnées Maven
    let url = build_maven_url(maven_coords)?;

    lighty_core::trace_debug!(
        loader = "neoforge",
        artifact = %maven_coords,
        "Downloading processor dependency"
    );

    if let Some(parent) = path.parent() {
        mkdir!(parent);
    }

    download_file_untracked(&url, &path)
        .await
        .map_err(|e| QueryError::Conversion {
            message: format!("Failed to download {}: {}", maven_coords, e),
        })?;

    Ok(path)
}

/// Construit l'URL de téléchargement pour un artifact Maven à partir de ses coordonnées
fn build_maven_url(maven_coords: &str) -> Result<String> {
    let parts: Vec<&str> = maven_coords.split(':').collect();
    if parts.len() < 3 {
        return Err(QueryError::Conversion {
            message: format!("Invalid Maven coordinate: {}", maven_coords),
        });
    }

    let group = parts[0].replace('.', "/");
    let artifact = parts[1];

    // Gérer les différentes variantes de coordonnées Maven
    let (version, classifier, extension) = if parts.len() >= 4 {
        // Format: group:artifact:version:classifier[@extension]
        let version = parts[2];
        let last_part = parts[3];

        if let Some((clf, ext)) = last_part.split_once('@') {
            (version, Some(clf), ext)
        } else {
            (version, Some(last_part), "jar")
        }
    } else {
        // Format: group:artifact:version[@extension]
        let version_part = parts[2];

        if let Some((ver, ext)) = version_part.split_once('@') {
            (ver, None, ext)
        } else {
            (version_part, None, "jar")
        }
    };

    let filename = if let Some(clf) = classifier {
        format!("{}-{}-{}.{}", artifact, version, clf, extension)
    } else {
        format!("{}-{}.{}", artifact, version, extension)
    };

    // Construire l'URL de téléchargement à partir des coordonnées Maven
    let base_url = "https://maven.neoforged.net/releases";
    let url = format!(
        "{}/{}/{}/{}/{}",
        base_url, group, artifact, version, filename
    );

    Ok(url)
}

/// Extrait un fichier du JAR de l'installer et retourne le chemin vers le fichier extrait
async fn extract_and_resolve(context: &ProcessorContext, internal_path: &str) -> Result<String> {
    let file_name = internal_path
        .split('/')
        .last()
        .ok_or_else(|| QueryError::Conversion {
            message: format!("Invalid internal path: {}", internal_path),
        })?;

    // Construire le chemin de sortie pour l'extraction
    let output_path = context
        .libraries_dir
        .join("net")
        .join("neoforged")
        .join("installer-extracts")
        .join(&context.minecraft_version)
        .join(file_name);

    if !output_path.exists() {
        context
            .extract_installer_file(internal_path, &output_path)
            .await?;
    }

    Ok(output_path.to_string_lossy().to_string())
}

/// Vérifie si un processor doit être sauté en fonction de ses outputs
fn should_skip_processor(context: &ProcessorContext, processor: &Processor) -> Result<bool> {
    if processor.outputs.is_empty() {
        return Ok(false);
    }

    // Vérifier chaque output : s'il n'existe pas ou si le hash ne correspond pas, on doit exécuter le processor
    for (output_path_pattern, expected_hash_pattern) in &processor.outputs {
        let output_path = context.substitute(output_path_pattern)?;
        let expected_hash = context.substitute(expected_hash_pattern)?;

        let expected_hash = expected_hash.trim_matches('\'').trim_matches('"');
        let path = PathBuf::from(&output_path);

        if !path.exists() {
            return Ok(false);
        }

        match lighty_core::verify_file_sha1_sync(&path, expected_hash) {
            Ok(true) => continue,
            _ => return Ok(false),
        }
    }

    Ok(true)
}

/// Extrait la Main-Class d'un JAR en lisant son MANIFEST.MF
fn extract_main_class(jar_path: &Path) -> Result<String> {
    let file = File::open(jar_path).map_err(|e| QueryError::Conversion {
        message: format!("Failed to open JAR: {}", e),
    })?;

    let mut archive = ZipArchive::new(file).map_err(|e| QueryError::Conversion {
        message: format!("Failed to open ZIP archive: {}", e),
    })?;

    let mut manifest_file =
        archive
            .by_name("META-INF/MANIFEST.MF")
            .map_err(|_| QueryError::MissingField {
                field: "META-INF/MANIFEST.MF in processor JAR".to_string(),
            })?;

    let mut contents = String::new();
    manifest_file
        .read_to_string(&mut contents)
        .map_err(|e| QueryError::Conversion {
            message: format!("Failed to read MANIFEST.MF: {}", e),
        })?;

    // Rechercher la ligne "Main-Class: " et extraire la classe principale
    for line in contents.lines() {
        if let Some(main_class) = line.strip_prefix("Main-Class:") {
            return Ok(main_class.trim().to_string());
        }
    }

    Err(QueryError::MissingField {
        field: "Main-Class in MANIFEST.MF".to_string(),
    })
}

