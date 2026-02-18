// Cet exemple montre comment nettoyer les fichiers non autorisés
// dans les dossiers du jeu (mods, libraries, natives) avant le lancement.
//
// Le nettoyage supprime automatiquement:
// - Les mods non autorisés
// - Les libraries non autorisées
// - Les natives non autorisées
// - Tout fichier non présent dans l'allowlist

use lighty_launcher::prelude::*;
use lighty_launcher::launch::{
    create_files_allowlist,
    cleanup_game_directories,
    cleanup_unauthorized_files,
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

    let launcher_dir = AppState::get_project_dirs();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     Démonstration du nettoyage de fichiers non autorisés     ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    // === ÉTAPE 1: Définir l'allowlist des fichiers autorisés ===
    println!("\n[1/3] Créer l'allowlist des fichiers autorisés...");

    let allowed_file_paths = vec![
        // Mods spécifiques - exactement ces fichiers
        "optifine.jar",
        "jei.jar",
        "sodium.jar",
        "modmenu.jar",
        
        // Libraries avec patterns
        "libraries/commons-*.jar",
        "libraries/gson-*.jar",
        "libraries/guava-*.jar",
        
        // Natives avec patterns
        "natives/lwjgl*.jar",
        "natives/lwjgl-*-natives-*.jar",
        
        // Fichiers config
        "*.json",
        "config/*.yaml",
    ];

    let allowlist = create_files_allowlist(&allowed_file_paths);
    
    println!("✓ Allowlist créée avec {} fichiers/patterns autorisés", allowlist.len());
    println!("  Mods autorisés: optifine.jar, jei.jar, sodium.jar, modmenu.jar");
    println!("  Libraries: commons-*.jar, gson-*.jar, guava-*.jar");
    println!("  Natives: lwjgl*.jar, lwjgl-*-natives-*.jar");
    println!("  Config: *.json, config/*.yaml");

    // === ÉTAPE 2: Démonstration du nettoyage sur un dossier fictif ===
    println!("\n[2/3] Exemple de nettoyage d'un dossier...");
    
    // On crée un dossier temporaire pour la démo
    let temp_dir = std::path::PathBuf::from("./test_cleanup");
    
    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir)?;
        
        // Créer des fichiers de test (autorisés et non autorisés)
        std::fs::write(temp_dir.join("optifine.jar"), "test")?;
        std::fs::write(temp_dir.join("jei.jar"), "test")?;
        std::fs::write(temp_dir.join("malicious-mod.jar"), "test")?;  // Non autorisé
        std::fs::write(temp_dir.join("config.json"), "test")?;
        
        println!("  Fichiers créés dans {}:", temp_dir.display());
        println!("    ✓ optifine.jar (autorisé)");
        println!("    ✓ jei.jar (autorisé)");
        println!("    ✗ malicious-mod.jar (NON autorisé - sera supprimé)");
        println!("    ✓ config.json (autorisé)");
    }

    println!("\n  Nettoyage en cours...");
    match cleanup_unauthorized_files(temp_dir.clone(), &allowlist).await {
        Ok(count) => {
            println!("✓ {} fichier(s) non autorisé(s) supprimé(s)", count);
            
            // Afficher les fichiers restants
            println!("\n  Fichiers restants après nettoyage:");
            for entry in std::fs::read_dir(&temp_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        println!("    ✓ {}", name.to_string_lossy());
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Erreur lors du nettoyage: {}", e);
        }
    }

    // Nettoyer le dossier de test
    let _ = std::fs::remove_dir_all(&temp_dir);

    // === ÉTAPE 3: Exemple d'utilisation avec les vrais dossiers du jeu ===
    println!("\n[3/3] Exemple d'utilisation avec le répertoire du jeu...");

    let game_dir = launcher_dir.data_local_dir().join("game");

    if game_dir.exists() {
        println!("  Nettoyage des répertoires du jeu:");
        println!("    - {}/mods", game_dir.display());
        println!("    - {}/libraries", game_dir.display());
        println!("    - {}/natives", game_dir.display());

        match cleanup_game_directories(&game_dir, &allowlist).await {
            Ok((mods_deleted, libs_deleted, natives_deleted)) => {
                println!("\n✓ Nettoyage complété:");
                if mods_deleted > 0 {
                    println!("  - {} fichiers non autorisés supprimés du dossier mods", mods_deleted);
                }
                if libs_deleted > 0 {
                    println!("  - {} fichiers non autorisés supprimés du dossier libraries", libs_deleted);
                }
                if natives_deleted > 0 {
                    println!("  - {} fichiers non autorisés supprimés du dossier natives", natives_deleted);
                }
                if mods_deleted == 0 && libs_deleted == 0 && natives_deleted == 0 {
                    println!("  - Aucun fichier non autorisé trouvé ✓");
                }
            }
            Err(e) => {
                println!("✗ Erreur lors du nettoyage: {}", e);
            }
        }
    } else {
        println!("  (Le répertoire du jeu n'existe pas, nettoyage ignoré)");
    }

    // === RÉSUMÉ ===
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                     Résumé                                  ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Le nettoyage automatique:                                  ║");
    println!("║ 1. Scanne récursivement les dossiers du jeu                ║");
    println!("║ 2. Identifie les fichiers non autorisés                    ║");
    println!("║ 3. Les supprime automatiquement                            ║");
    println!("║ 4. S'exécute avant le lancement du jeu                     ║");
    println!("║                                                            ║");
    println!("║ Utilisation:                                               ║");
    println!("║ - cleanup_unauthorized_files() pour un dossier             ║");
    println!("║ - cleanup_game_directories() pour tous les dossiers        ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    Ok(())
}
