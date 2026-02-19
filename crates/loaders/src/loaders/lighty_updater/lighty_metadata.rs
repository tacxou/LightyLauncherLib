use serde::{Deserialize, Serialize};

// Response du serveur contenant la liste des serveurs disponibles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServersResponse {
    servers: Vec<ServerInfo>,
}

impl ServersResponse {
    /// Retourne la liste des serveurs disponibles
    #[allow(dead_code)]
    pub fn servers(&self) -> &[ServerInfo] {
        &self.servers
    }

    /// Trouve un serveur par son nom
    pub fn find_by_name(&self, name: &str) -> Option<&ServerInfo> {
        self.servers.iter().find(|s| s.name == name)
    }
}

// Info d'un serveur spécifique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    name: String,
    loader: String,
    loader_version: String,
    minecraft_version: String,
    url: String,
    last_update: String,
}

impl ServerInfo {
    /// Retourne le nom du serveur
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Retourne le type de loader (vanilla, fabric, quilt, etc.)
    pub fn loader(&self) -> &str {
        &self.loader
    }

    /// Retourne la version du loader
    pub fn loader_version(&self) -> &str {
        &self.loader_version
    }

    /// Retourne la version Minecraft
    pub fn minecraft_version(&self) -> &str {
        &self.minecraft_version
    }

    /// Retourne l'URL du serveur
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Retourne la date de dernière mise à jour
    pub fn last_update(&self) -> &str {
        &self.last_update
    }
}

//STRUCTURE OF LIGHTY_UPDATER METADATA
/// Tous les champs sont optionnels car le serveur LightyUpdater
/// peut ne pas fournir toutes les métadonnées. Les champs manquants
/// seront complétés par le loader de base (vanilla/fabric/quilt/etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LightyMetadata {
    #[serde(skip)]  // Ne pas sérialiser server_info car il vient d'une autre requête
    pub server_info: Option<ServerInfo>,
    pub main_class: Option<MainClass>,
    pub java_version: Option<JavaVersion>,
    pub arguments: Option<Arguments>,
    pub libraries: Option<Vec<Library>>,
    pub natives: Option<Vec<Native>>,
    pub client: Option<Client>,
    pub assets: Option<Vec<Asset>>,
    pub mods: Option<Vec<Mod>>,
}

impl Default for LightyMetadata {
    fn default() -> Self {
        Self {
            server_info: None,
            main_class: None,
            java_version: None,
            arguments: None,
            libraries: None,
            natives: None,
            client: None,
            assets: None,
            mods: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainClass {
    pub main_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersion {
    pub major_version: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    pub game: Vec<String>,
    pub jvm: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub name: String,
    pub url: String,
    pub path: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Native {
    pub name: String,
    pub url: String,
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub os: String,  // "windows", "linux", or "macos"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub name: String,
    pub url: String,
    pub path: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub hash: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}
