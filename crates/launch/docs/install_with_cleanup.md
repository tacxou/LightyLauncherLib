# Installation avec Nettoyage Automatique

## 🎯 Objectif

La fonction `install_with_cleanup()` exécute l'installation du jeu ET nettoie automatiquement tous les fichiers non autorisés après l'installation complète.

## 📊 Architecture

```
┌─────────────────────────────────────┐
│  install_with_cleanup()             │
│  Nouveaux paramètres:               │
│  - allowed_files: Option<HashSet>   │  ← Allowlist optionnelle
├─────────────────────────────────────┤
│  1. Installation standard            │
│     (version.install())              │
├─────────────────────────────────────┤
│  2. Nettoyage automatique (SI oui)  │
│     cleanup_game_directories()       │
│     Scan: mods/,                    │
│           libraries/,               │
│           natives/                  │
│                                     │
│  Supprime les fichiers en NON dans  │
│  l'allowlist                        │
└─────────────────────────────────────┘
```

## 🔧 Utilisation

### Option 1: Installation standard (sans nettoyage)
```rust
version.install(&builder, event_bus).await?;
```

### Option 2: Installation + nettoyage automatique
```rust
use lighty::prelude::*;
use std::collections::HashSet;

// Créer l'allowlist
let allowed = create_files_allowlist(&[
    "optifine*.jar",      // Optifine
    "jei*.jar",           // JEI
    "sodium*.jar",        // Sodium
    "*.json",             // Fichiers config
    "libraries/*.jar",    // Toutes les libraries
]);

// Installation avec nettoyage
install_with_cleanup(
    &version,
    &builder,
    Some(&allowed),  // Passiez l'allowlist
    event_bus
).await?;
```

## 📝 Signature de la fonction

```rust
pub async fn install_with_cleanup<T: VersionInfo + Installer>(
    version: &T,                              // La version à installer
    builder: &Version,                        // Configuration du builder
    allowed_files: Option<&HashSet<String>>,  // L'allowlist (optionnelle)
    event_bus: Option<&EventBus>,             // Bus d'événements
) -> InstallerResult<()>
```

## 🎁 Bénéfices de install_with_cleanup()

| Bénéfice | Description |
|----------|-------------|
| **Sécurité** | Empêche les mods malveillants d'être lancés |
| **Contrôle** | Seuls les mods approuvés sont présents dans l'instance |
| **Automatisation** | S'exécute sans intervention de l'utilisateur |
| **Flexibilité** | Allowlist optionnelle (None = pas de nettoyage) |
| **Robustesse** | Les erreurs de nettoyage ne cassent pas l'installation |
| **Traçabilité** | Rapporte les fichiers supprimés dans les logs |

## 📚 Patterns d'allowlist supportés

### Patterns simples
```rust
"optifine.jar"           // Nom exact
"optifine*.jar"          // Wildcard: optifine-1.21.1.jar
"**/config/*"            // N'importe quel niveau: dir/config/*
```

### Patterns de répertoires
```rust
"mods/*"                 // Tous les fichiers de mods/
"libraries/*"            // Tous les fichiers de libraries/
"natives/*"              // Tous les fichiers de natives/
"config/**/settings*"    // Fichiers settings dans config/
```

### Patterns complexes
```rust
"libraries/commons-*.jar"    // Exactement commons-*.jar dans libraries/
"natives/lwjgl-*.jar"        // Natives avec préfixe lwjgl-
"mods/*-1.21*.jar"           // Mods avec version 1.21
```

## ⚙️ Comment ça fonctionne

### Phase 1: Installation standard
```
install_with_cleanup()
    ↓
version.install(&builder)
    ├─ Créer les répertoires (mods/, libraries/, natives/)
    ├─ Télécharger les fichiers manquants
    ├─ Vérifier les SHA1
    └─ Extraire les natives
```

### Phase 2: Nettoyage (SI allowed_files = Some)
```
install_with_cleanup()
    ↓
cleanup_game_directories(&game_dir, &allowlist)
    ├─ Parcours mods/
    │   └─ Pour chaque fichier
    │      ├─ Est dans allowlist? NON → Supprimer ❌
    │      └─ Est dans allowlist? OUI → Garder ✓
    │
    ├─ Parcours libraries/
    │   └─ (même logique)
    │
    └─ Parcours natives/
        └─ (même logique)
```

### Phase 3: Rapport
```
Logs affichent:
  ✓ 2 fichier(s) non autorisé(s) supprimé(s)
  ✗ malicious-mod.jar removed
  ✗ unknown-library.jar removed
```

## 🔄 Flux de lancement recommandé

Pour un launcher utilisant cette fonction:

```
┌─────────────────────┐
│  Clic "Lancer"      │
└──────────┬──────────┘
           ▼
┌─────────────────────┐
│  install_with_      │
│  cleanup()          │
│                     │
│  1. Installation    │
│  2. Nettoyage      │
└──────────┬──────────┘
           ▼
┌─────────────────────┐
│  Tous les fichiers  │
│  non-autorisés     │
│  sont supprimés ✓  │
└──────────┬──────────┘
           ▼
┌─────────────────────┐
│  Lancer Minecraft   │
│  (instance propre)  │
└─────────────────────┘
```

## 🛡️ Gestion des erreurs

### Erreurs d'installation
Si l'installation échoue → `install_with_cleanup()` échoue
```rust
if let Err(e) = install_with_cleanup(...).await {
    eprintln!("Erreur installation: {}", e);
    // Installation non complétée, pas de nettoyage
}
```

### Erreurs de nettoyage
Si le nettoyage échoue → Installation réussit (log warning seulement)
```rust
// Les erreurs lors du nettoyage sont traitées comme:
// - Logged comme WARNING
// - NE causent PAS l'échec de la fonction
// - Permet de lancer le jeu même si nettoyage échoue
```

## 📊 Exemple d'allowlist complète

```rust
let allowed_mods = vec![
    // Optimisation
    "optifine*.jar",
    "sodium*.jar",
    "indium*.jar",
    "phosphor*.jar",
    
    // Utilitaires
    "jei*.jar",
    "modmenu*.jar",
    "cloth-config*.jar",
    
    // Contenu
    "minecraft-addons/*.jar",
    
    // Libraries (mineties partagées)
    "libraries/org/lwjgl/**",
    "libraries/com/google/**",
    "libraries/com/mojang/**",
    
    // Natives (obligatoires pour Minecraft)
    "natives/*.dll",       // Windows
    "natives/*.dylib",     // macOS
    "natives/*.so",        // Linux
    
    // Fichiers de configuration
    "*.json",
    "*.yaml",
    "config/*",
    "server.properties",
];

let allowlist = create_files_allowlist(&allowed_mods);
```

## 🔍 Vérification du comportement

### Ce qui est supprimé
```
❌ mods/malicious-mod.jar            (pas dans allowlist)
❌ mods/random-addon-1.0.jar         (pas dans allowlist)
❌ libraries/unknown-lib.jar          (pas dans allowlist)
❌ natives/suspicious-native.dll      (pas dans allowlist)
```

### Ce qui est conservé
```
✅ mods/optifine-1.21.1_HQ.jar       (dans allowlist)
✅ mods/jei-1-21_1-19.3.7.jar        (dans allowlist)
✅ libraries/commons-lang3.jar       (dans allowlist: commons-*.jar)
✅ natives/lwjgl-3.2.1.jar           (dans allowlist: natives/*)
✅ config/options.json               (dans allowlist: *.json)
```

## 📋 Checklist d'implémentation

Pour utiliser `install_with_cleanup()` dans votre launcher:

- [ ] Créer l'allowlist des mods autorisés
- [ ] Créer l'allowlist des libraries si nécessaire
- [ ] Importer `install_with_cleanup` et `create_files_allowlist`
- [ ] Remplacer `version.install()` par `install_with_cleanup()`
- [ ] Passer l'allowlist dans le paramètre `allowed_files`
- [ ] Tester avec des fichiers non-autorisés
- [ ] Vérifier les logs pour confirmer le nettoyage
- [ ] Documenter les mods autorisés pour les utilisateurs

## 🚀 Performance

- **Parcours des répertoires**: VecDeque itérative (O(n) fichiers)
- **Vérification d'allowlist**: HashSet O(1) par fichier
- **Suppression**: Async tokio::fs::remove_file
- **Temps total**: ~100-500ms pour 100 fichiers (selon disque)

## 💡 Cas d'usage

### 1. Launcher officiel (recommandé)
```rust
// Tous les mods doivent être dans la liste officielle
install_with_cleanup(&version, &builder, Some(&official_mods), event_bus).await?;
```

### 2. Launcher semi-ouvert
```rust
// Certains mods supplémentaires sont tolérés
install_with_cleanup(&version, &builder, Some(&extended_allowlist), event_bus).await?;
```

### 3. Installation standard (pas de contrôle)
```rust
// Pas de nettoyage, l'utilisateur peut faire ce qu'il veut
version.install(&builder, event_bus).await?;
```

## ✅ Tests

Voir les exemples:
- `examples/install_with_cleanup_demo.rs` - Démonstration complète
- `examples/cleanup_unauthorized.rs` - Test de nettoyage
- `examples/mods_verification.rs` - Test de vérification

Exécuter les tests:
```bash
cargo run --example install_with_cleanup_demo --all-features
cargo run --example cleanup_unauthorized --all-features
```

## 📎 Voir aussi

- [allowlist.md](./allowlist.md) - Documentation sur les allowlists
- [how-to-use.md](./how-to-use.md) - Guide d'utilisation complet
- [exports.md](./exports.md) - Fonctions publiques exportées
