# Vérification des Mods

La vérification des mods empêche l'installation de mods non autorisés en vérifiant que tous les mods à installer sont présents dans une liste blanche (whitelist).

## Utilisation

### 1. Créer une liste blanche de mods autorisés

```rust
use lighty_launcher::installer::create_mods_whitelist;

// Définir la liste des mods autorisés
let allowed_mod_names = vec![
    "optifine",
    "jei",
    "modmenu",
    "sodium",
];

// Créer la whitelist
let whitelist = create_mods_whitelist(
    allowed_mod_names.iter().map(|s| *s).collect::<Vec<_>>().as_slice()
);
```

### 2. Collecter les tâches de mod avec vérification

```rust
use lighty_launcher::installer::collect_mod_tasks_verified;
use lighty_loaders::types::Mods;

// Vos mods du serveur
let mods = vec![
    Mods {
        name: "optifine".to_string(),
        url: Some("https://example.com/optifine.jar".to_string()),
        path: Some("optifine.jar".to_string()),
        sha1: Some("abc123...".to_string()),
        size: Some(12345),
    },
    Mods {
        name: "jei".to_string(),
        url: Some("https://example.com/jei.jar".to_string()),
        path: Some("jei.jar".to_string()),
        sha1: Some("def456...".to_string()),
        size: Some(54321),
    },
];

// Collecter les tâches avec vérification
match collect_mod_tasks_verified(version_info, &mods, &whitelist).await {
    Ok(tasks) => println!("✓ Tous les mods sont autorisés"),
    Err(e) => eprintln!("✗ Erreur: {}", e),
}
```

## Gestion des erreurs

Si un mod n'est pas autorisé, vous recevrez une erreur `UnauthorizedMods`:

```rust
use lighty_launcher::errors::InstallerError;

match collect_mod_tasks_verified(version_info, &mods, &whitelist).await {
    Ok(tasks) => {
        // Procéder à l'installation
    }
    Err(InstallerError::UnauthorizedMods(mod_list)) => {
        eprintln!("Mods non autorisés détectés: {}", mod_list);
        // Interrompre l'installation
    }
    Err(e) => {
        eprintln!("Erreur pendant la vérification: {}", e);
    }
}
```

## Sécurité

Le vérificateur compare chaque mod par son nom contre la liste blanche. Cela garantit que:

- ✓ Aucun mod non autorisé ne sera installé
- ✓ La liste blanche est vérifiée avant le téléchargement
- ✓ Des messages d'erreur clairs indiquent les mods problématiques
- ✓ Le process d'installation s'arrête avant de télécharger des fichiers non autorisés

## API

### `create_mods_whitelist(allowed_mod_names: &[&str]) -> HashSet<String>`

Crée une liste blanche à partir d'un slice de noms de mods.

### `verify_mods_in_whitelist(mods: &[Mods], allowed_mods: &HashSet<String>) -> Result<(), InstallerError>`

Vérifie que tous les mods sono dans la liste blanche.

Retourne:
- `Ok(())` si tous les mods sont autorisés
- `Err(InstallerError::UnauthorizedMods(mod_list))` si des mods non autorisés sont trouvés

### `collect_mod_tasks_verified(version, mods, allowed_mods) -> Result<Vec<(String, PathBuf)>, InstallerError>`

Collecte les tâches de téléchargement des mods avec vérification préalable.

Retourne:
- `Ok(tasks)` avec la liste des tâches si tous les mods sont valides
- `Err(InstallerError::UnauthorizedMods(...))` si des mods ne sont pas autorisés
- Les autres erreurs possibles du vérificateur de fichiers
