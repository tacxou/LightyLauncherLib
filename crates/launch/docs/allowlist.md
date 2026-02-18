# Allowlist générique de fichiers

L'allowlist est une **liste blanche** de fichiers et chemins autorisés que les utilisateurs peuvent modifier sans risque de resynchronisation ou suppression.

## Concepts

- **Allowlist**: Liste des fichiers/chemins qui sont autorisés à être modifiés
- **Patterns**: Support des patterns glob pour les patterns de fichiers
- **Nettoyage automatique**: Supprime les fichiers non autorisés avant le lancement
- **Non-destructif sur allowlist**: Les fichiers dans l'allowlist ne seront jamais supprimés

## Caractéristiques

✅ Peut s'appliquer à tous les types de fichiers (mods, libraries, natives, assets, etc.)  
✅ Support des patterns glob simples (`*.jar`, `mods/*`, `libraries/commons-*`)  
✅ Protège les modifications personnalisées des utilisateurs  
✅ Prévient l'ajout de fichiers malveillants non autorisés  
✅ **Nettoyage automatique des fichiers non autorisés**  

## Patterns supportés

| Pattern | Description | Exemple |
|---------|-------------|---------|
| `exact_file.jar` | Match exact | `optifine.jar` |
| `*.jar` | Any file ending with | `*.jar` matches `mod.jar` |
| `mods/*` | Any file in directory | `mods/*` matches `mods/my-mod.jar` |
| `libraries/commons-*.jar` | Prefix wildcard | matches `commons-lang3.jar`, `commons-io-2.8.jar` |
| `**/config` | Recursive (ends with) | `**/config` matches `any/path/to/config` |
| `*.json` | Extension pattern | matches any JSON file |

## Utilisation

### 1. Créer une allowlist

```rust
use lighty_launcher::launch::create_files_allowlist;

let allowed_paths = vec![
    // Mods spécifiques
    "optifine.jar",
    "jei.jar",
    "sodium.jar",
    
    // Patterns pour libraries
    "libraries/commons-*.jar",
    "libraries/gson-*.jar",
    
    // Fichiers config
    "config/*.json",
    "*.yaml",
    
    // Tout fichier dans un dossier
    "mods/*",
];

let allowlist = create_files_allowlist(&allowed_paths);
```

### 2. Vérifier des fichiers avant installation

```rust
use lighty_launcher::launch::verify_files_in_allowlist;

let files_to_add = vec![
    "optifine.jar",
    "jei.jar",
    "config/settings.json",
];

match verify_files_in_allowlist(&files_to_add, &allowlist) {
    Ok(_) => println!("✓ Tous les fichiers sont autorisés"),
    Err(e) => eprintln!("✗ Erreur: {}", e),
}
```

### 3. Nettoyer les fichiers non autorisés

```rust
use lighty_launcher::launch::cleanup_unauthorized_files;

// Nettoyer un dossier spécifique
match cleanup_unauthorized_files(game_dir.join("mods"), &allowlist).await {
    Ok(count) => println!("✓ {} fichiers non autorisés supprimés", count),
    Err(e) => eprintln!("✗ Erreur: {}", e),
}
```

### 4. Nettoyer tous les dossiers du jeu

```rust
use lighty_launcher::launch::cleanup_game_directories;

// Nettoie mods/, libraries/, et natives/
match cleanup_game_directories(&game_dir, &allowlist).await {
    Ok((mods_deleted, libs_deleted, natives_deleted)) => {
        if mods_deleted > 0 {
            println!("  - {} mods supprimés", mods_deleted);
        }
        if libs_deleted > 0 {
            println!("  - {} libraries supprimées", libs_deleted);
        }
        if natives_deleted > 0 {
            println!("  - {} natives supprimées", natives_deleted);
        }
    }
    Err(e) => eprintln!("✗ Erreur: {}", e),
}
```

### 5. Utiliser dans le flux d'installation

#### Pour les mods (avec vérification avant installation)

```rust
use lighty_launcher::launch::collect_mod_tasks_with_allowlist;

match collect_mod_tasks_with_allowlist(version, &mods, &allowlist).await {
    Ok(tasks) => {
        // Procéder au téléchargement
    }
    Err(e) => {
        eprintln!("Mods non autorisés: {}", e);
    }
}
```

#### Pour les libraries (avec vérification avant installation)

```rust
use lighty_launcher::launch::collect_library_tasks_with_allowlist;

match collect_library_tasks_with_allowlist(version, &libraries, &allowlist).await {
    Ok(tasks) => {
        // Procéder au téléchargement
    }
    Err(e) => {
        eprintln!("Libraries non autorisées: {}", e);
    }
}
```

#### Pour les natives (avec vérification avant installation)

```rust
use lighty_launcher::launch::collect_native_tasks_with_allowlist;

match collect_native_tasks_with_allowlist(version, &natives, &allowlist).await {
    Ok((download_tasks, extract_paths)) => {
        // Procéder au téléchargement et extraction
    }
    Err(e) => {
        eprintln!("Natives non autorisées: {}", e);
    }
}
```

## Flux de lancement recommandé

```rust
use lighty_launcher::launch::{
    create_files_allowlist,
    cleanup_game_directories,
    collect_mod_tasks_with_allowlist,
};

// 1. Créer l'allowlist
let allowlist = create_files_allowlist(&allowed_paths);

// 2. Nettoyer les fichiers existants non autorisés
cleanup_game_directories(&game_dir, &allowlist).await?;

// 3. Collecter les tâches d'installation avec vérification
let mod_tasks = collect_mod_tasks_with_allowlist(version, &mods, &allowlist).await?;

// 4. Procéder au lancement
version.install(/* ... */).await?;
version.launch(/* ... */).await?;
```

## Gestion des erreurs

```rust
use lighty_launcher::launch::InstallerError;

match verify_files_in_allowlist(&files, &allowlist) {
    Ok(_) => {
        // Tous les fichiers sont autorisés
    }
    Err(InstallerError::FilesNotAllowed(file_list)) => {
        eprintln!("Fichiers non autorisés: {}", file_list);
        // Ne pas procéder à l'installation
    }
    Err(e) => {
        eprintln!("Erreur: {}", e);
    }
}
```

## Exemples

### Exemple 1: Vérification simple

Voir [examples/mods_verification.rs](../../examples/mods_verification.rs)

Teste:
1. Fichiers autorisés → Acceptés
2. Un fichier non autorisé → Rejeté
3. Tous non autorisés → Rejetés
4. Patterns directory → Acceptés
5. Fichiers JSON par pattern → Acceptés

Exécuter:
```bash
cargo run --example mods_verification --all-features
```

### Exemple 2: Nettoyage automatique

Voir [examples/cleanup_unauthorized.rs](../../examples/cleanup_unauthorized.rs)

Démontre:
1. Création d'une allowlist
2. Nettoyage d'un dossier de test
3. Utilisation avec les vrais dossiers du jeu

Exécuter:
```bash
cargo run --example cleanup_unauthorized --all-features
```

## Sécurité

La allowlist fournit:

- ✅ Prévention des injections de fichiers malveillants
- ✅ Vérification avant tout téléchargement/extraction
- ✅ Arrêt immédiat si un fichier n'est pas autorisé
- ✅ Nettoyage automatique des fichiers non autorisés existants
- ✅ Messages d'erreur clairs listant les fichiers problématiques
- ✅ Support multi-fichier avec détection d'erreurs cumulées
- ✅ Gestion sûre et efficace de la suppression de fichiers

## API complète

### Création et vérification

- `create_files_allowlist(allowed_paths: &[&str]) -> HashSet<String>` - Crée une allowlist
- `verify_files_in_allowlist(file_paths: &[&str], allowed_files: &HashSet<String>) -> Result<(), InstallerError>` - Vérifie les fichiers

### Nettoyage

- `cleanup_unauthorized_files(dir_path: PathBuf, allowed_files: &HashSet<String>) -> Result<usize>` - Nettoie un dossier
- `cleanup_game_directories(game_dir: &PathBuf, allowed_files: &HashSet<String>) -> Result<(usize, usize, usize)>` - Nettoie tous les dossiers du jeu

### Installation avec allowlist

- `collect_mod_tasks_with_allowlist(version, mods, allowed_files) -> Result<Vec<...>>` - Collecte les tâches de mods
- `collect_library_tasks_with_allowlist(version, libraries, allowed_files) -> Result<Vec<...>>` - Collecte les tâches de libraries
- `collect_native_tasks_with_allowlist(version, natives, allowed_files) -> Result<(Vec<...>, Vec<...>)>` - Collecte les tâches de natives

