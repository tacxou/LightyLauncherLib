// Cet exemple montre comment utiliser la fonction install_with_cleanup
// qui effectue l'installation et le nettoyage automatiquement.
//
// Utilisation:
// cargo run --example install_with_cleanup_demo --features="neoforge tracing events"

use lighty_launcher::prelude::*;
use lighty_launcher::launch::create_files_allowlist;

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

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  Démonstration de l'installation avec nettoyage automatique  ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    // === Créer l'allowlist des fichiers autorisés ===
    println!("\n[1/3] Créer l'allowlist des fichiers autorisés...");

    let allowed_file_paths = vec![
        // Optifine
        "optifine*.jar",
        
        // JEI et addons
        "jei*.jar",
        
        // Sodium et dépendances
        "sodium*.jar",
        "indium*.jar",
        
        // Modmenu
        "modmenu*.jar",
        
        // Patterns pour les libraries communes
        "libraries/commons-*.jar",
        "libraries/guava-*.jar",
        "libraries/gson-*.jar",
        "libraries/log4j-*.jar",
        
        // Natives (lwjgl)
        "natives/lwjgl*.jar",
        "natives/lwjgl-*-natives-*.jar",
        
        // Fichiers config
        "*.json",
        "*.yaml",
        "config/*",
    ];

    let allowlist = create_files_allowlist(&allowed_file_paths);
    
    println!("✓ Allowlist créée avec {} fichiers/patterns autorisés", allowlist.len());

    // === Afficher le flux d'installation standard ===
    println!("\n[2/3] Flux d'installation standard:");
    println!("  1. Créer les répertoires");
    println!("  2. Vérifier les fichiers existants");
    println!("  3. Télécharger les fichiers manquants");
    println!("  4. Extraire les natives");
    println!("  ▸ 5. NETTOYER les fichiers non autorisés ← NOUVEAU!");

    // === Expliquer l'utilisation ===
    println!("\n[3/3] Utilisation de `install_with_cleanup`:");
    println!();
    println!("  // Approche 1: Installation standard (sans nettoyage)");
    println!("  version.install(&builder, event_bus).await?;");
    println!();
    println!("  // Approche 2: Installation avec nettoyage automatique");
    println!("  install_with_cleanup(");
    println!("      &version,");
    println!("      &builder,");
    println!("      Some(&allowlist),  // Passez l'allowlist");
    println!("      event_bus");
    println!("  ).await?;");
    println!();
    println!("  // Avantages:");
    println!("  ✓ Protège contre les ajouts manuels de mods non autorisés");
    println!("  ✓ Maintient une instance de jeu 'propre' et contrôlée");
    println!("  ✓ S'exécute automatiquement avant chaque lancement");
    println!("  ✓ Signale les fichiers supprimés via les logs");

    // === Afficher les fichiers qui seraient supprimés ===
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  Fichiers qui seraient supprimés (exemple):                ║");
    println!("╠════════════════════════════════════════════════════════════╣");

    let examples_to_delete = vec![
        "malicious-mod.jar",
        "mods/backdoor.jar",
        "random-hack-library.jar",
        "libraries/unknown-lib-1.0.jar",
        "natives/evil-native.dll",
    ];

    for file in &examples_to_delete {
        println!("║  ✗ {}  (non autorisé)", file);
    }

    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║  Fichiers qui seraient conservés (autorisés):             ║");
    println!("╠════════════════════════════════════════════════════════════╣");

    let examples_to_keep = vec![
        "optifine-1.21.1_HQ.jar",
        "jei-1-21_1-19.3.7.jar",
        "sodium-0.5.11.jar",
        "modmenu-13.0.0-beta.1.jar",
        "libraries/commons-lang3-3.12.0.jar",
        "config/options.json",
    ];

    for file in &examples_to_keep {
        println!("║  ✓ {}  (autorisé)", file);
    }

    println!("╚════════════════════════════════════════════════════════════╝");

    // === Résumé ===
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                      Résumé                                ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Flux d'installation avec nettoyage automatique:           ║");
    println!("║                                                            ║");
    println!("║ install_with_cleanup()");
    println!("║  ├─ Installation standard                               ║");
    println!("║  │  ├─ Création des répertoires                         ║");
    println!("║  │  ├─ Vérification SHA1                                ║");
    println!("║  │  ├─ Téléchargement des fichiers                      ║");
    println!("║  │  └─ Extraction des natives                           ║");
    println!("║  └─ Nettoyage automatique (NOUVEAU!)                    ║");
    println!("║     ├─ Scanne mods/, libraries/, natives/               ║");
    println!("║     ├─ Supprime les fichiers non autorisés              ║");
    println!("║     └─ Rapporte les suppressions                        ║");
    println!("║                                                            ║");
    println!("║ Résultat: Une instance de jeu sûre et contrôlée ✓       ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    Ok(())
}
