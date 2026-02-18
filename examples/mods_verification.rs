// Cet exemple montre comment utiliser l'allowlist générique pour contrôler
// quels fichiers peuvent être modifiés ou ajoutés au dossier du jeu.
//
// L'allowlist peut être appliquée à:
// - Les mods
// - Les libraries
// - Les natives
// - Tout autre fichier du jeu
//
// Les fichiers et chemins NON présents dans l'allowlist seront rejetés.

use lighty_launcher::prelude::*;
use lighty_launcher::launch::{
    create_files_allowlist,
    verify_files_in_allowlist,
    InstallerError,
};

const QUALIFIER: &str = "fr";
const ORGANIZATION: &str = ".LightyLauncher";
const APPLICATION: &str = "";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let _app_state = AppState::new(
        QUALIFIER.to_string(),
        ORGANIZATION.to_string(),
        APPLICATION.to_string(),
    )?;

    // === DÉFINIR L'ALLOWLIST DES FICHIERS AUTORISÉS ===
    let allowed_file_paths = vec![
        "optifine.jar",
        "jei.jar",
        "modmenu.jar",
        "sodium.jar",
        "rei.jar",
        "libraries/commons-lang*.jar",  // Pattern pour commons-lang3.jar
        "libraries/gson-*.jar",          // Pattern pour gson
        "*.json",                        // Pattern pour fichiers config
    ];

    let allowlist = create_files_allowlist(&allowed_file_paths);

    println!("✓ Allowlist créée avec {} fichiers/patterns autorisés", allowlist.len());
    println!("  Fichiers/patterns autorisés: {}", allowed_file_paths.join(", "));

    // === CAS 1: Fichiers autorisés ===
    println!("\n--- CAS 1: Tous les fichiers sont autorisés ---");
    let authorized_files = vec![
        "optifine.jar",
        "jei.jar",
    ];

    match verify_files(&authorized_files, &allowlist) {
        Ok(_) => println!("✓ Tous les fichiers sont autorisés"),
        Err(e) => println!("✗ Erreur: {}", e),
    }

    // === CAS 2: Un fichier non autorisé ===
    println!("\n--- CAS 2: Un fichier non autorisé ---");
    let mixed_files = vec![
        "optifine.jar",
        "malicious-mod.jar",
    ];

    match verify_files(&mixed_files, &allowlist) {
        Ok(_) => println!("✓ Tous les fichiers sont autorisés"),
        Err(e) => println!("✗ Erreur: {}", e),
    }

    // === CAS 3: Tous les fichiers sont non autorisés ===
    println!("\n--- CAS 3: Tous les fichiers sont non autorisés ---");
    let unauthorized_files = vec![
        "unauth-mod-1.jar",
        "unauth-mod-2.jar",
    ];

    match verify_files(&unauthorized_files, &allowlist) {
        Ok(_) => println!("✓ Tous les fichiers sont autorisés"),
        Err(e) => println!("✗ Erreur: {}", e),
    }

    // === CAS 4: Fichiers dans des dossiers autorisés ===
    println!("\n--- CAS 4: Fichiers dans des dossiers autorisés ---");
    let library_files = vec![
        "libraries/commons-lang3.jar",
        "libraries/gson-2.10.1.jar",
    ];

    match verify_files(&library_files, &allowlist) {
        Ok(_) => println!("✓ Tous les fichiers de libraries sont autorisés"),
        Err(e) => println!("✗ Erreur: {}", e),
    }

    // === CAS 5: Fichiers JSON autorisés ===
    println!("\n--- CAS 5: Fichiers JSON autorisés par pattern ---");
    let config_files = vec![
        "config.json",
        "settings.json",
    ];

    match verify_files(&config_files, &allowlist) {
        Ok(_) => println!("✓ Tous les fichiers JSON sont autorisés"),
        Err(e) => println!("✗ Erreur: {}", e),
    }

    // === CAS 6: Liste vide ===
    println!("\n--- CAS 6: Aucun fichier à vérifier ---");
    let empty_files: Vec<&str> = vec![];

    match verify_files(&empty_files, &allowlist) {
        Ok(_) => println!("✓ Aucun fichier à vérifier"),
        Err(e) => println!("✗ Erreur: {}", e),
    }

    println!("\n✓ Démonstration complète de l'allowlist");
    Ok(())
}

/// Fonction helper pour vérifier les fichiers
fn verify_files(files: &[&str], allowlist: &std::collections::HashSet<String>) 
    -> Result<(), InstallerError> {
    verify_files_in_allowlist(files, allowlist)
}
