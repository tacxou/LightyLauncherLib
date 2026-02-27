use chrono;
use std::future::Future;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::AuthError;

#[cfg(feature = "events")]
use lighty_event::EventBus;

pub type AuthResult<T> = Result<T, AuthError>;

/// Trait pour le rafraîchissement du token selon le mode d'authentification
#[async_trait::async_trait]
pub trait TokenRefreshable {
    async fn refresh_access_token(&self, profile: &UserProfile) -> AuthResult<UserProfile>;
}

/// User profile returned after successful authentication
#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    /// User ID (optional for offline mode)
    pub id: Option<u64>,

    /// Username
    pub username: String,

    /// Minecraft UUID (with dashes)
    pub uuid: String,

    /// Access token for session validation
    pub access_token: Option<String>,

    /// Refresh token for OAuth
    pub refresh_token: Option<String>,

    /// User email (optional)
    pub email: Option<String>,

    /// Email verification status
    pub email_verified: bool,

    /// User money/credits (for custom launchers)
    pub money: Option<f64>,

    /// User role/rank
    pub role: Option<UserRole>,

    /// Whether the account is banned
    pub banned: bool,

    /// Timestamp d'expiration du token (en secondes depuis epoch)
    pub expires_in: u64,

    /// Authentication provider
    pub provider: AuthProvider,

    /// Implémentation dynamique du refresh (totalement ignorée par Serde)
    #[serde(skip_serializing, skip_deserializing, default)]
    #[doc(hidden)]
    pub refresh_impl: Option<Arc<dyn TokenRefreshable + Send + Sync>>,

    /// Date et heure d'émission du profil (UTC)
    pub emited_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// User role/rank information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    /// Role name
    pub name: String,

    /// Role color (hex format: #RRGGBB)
    pub color: Option<String>,
}

/// Authentication provider type
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthProvider {
    /// Offline mode - no authentication
    Offline,

    /// Azuriom CMS authentication
    Azuriom {
        /// Base URL of the Azuriom instance (e.g., "https://example.com")
        base_url: String,
    },

    /// Microsoft/Xbox Live authentication
    Microsoft {
        /// OAuth client ID
        client_id: String,
    },

    /// Custom authentication endpoint
    Custom {
        /// Base URL of the custom auth API
        base_url: String,
    },
}

/// Core authentication trait
///
/// All authentication providers must implement this trait
pub trait Authenticator {
    /// Authenticate a user and return their profile
    ///
    /// # Arguments
    /// - `event_bus`: Optional event bus for emitting auth events
    ///
    /// # Returns
    /// - `Ok(UserProfile)` on success
    /// - `Err(AuthError)` on failure
    fn authenticate(
        &mut self,
        #[cfg(feature = "events")] event_bus: Option<&EventBus>,
    ) -> impl Future<Output = AuthResult<UserProfile>> + Send;

    /// Verify if a token is still valid
    ///
    /// # Arguments
    /// - `token`: The access token to verify
    ///
    /// # Returns
    /// - `Ok(UserProfile)` if token is valid
    /// - `Err(AuthError)` if token is invalid or expired
    fn verify(&self, token: &str) -> impl Future<Output = AuthResult<UserProfile>> + Send {
        async move {
            let _ = token;
            Err(AuthError::Custom("Verification not supported for this provider".into()))
        }
    }

    /// Logout and invalidate the token
    ///
    /// # Arguments
    /// - `token`: The access token to invalidate
    fn logout(&self, token: &str) -> impl Future<Output = AuthResult<()>> + Send {
        async move {
            let _ = token;
            Ok(())
        }
    }
}

/// Helper to generate UUID v5 from username (for offline mode)
///
/// Uses SHA1 hashing to generate a deterministic UUID from the username.
/// This ensures the same username always produces the same UUID.
///
/// # Arguments
/// - `username`: The username to generate a UUID for
///
/// # Returns
/// A UUID v5 string in the format: xxxxxxxx-xxxx-5xxx-yxxx-xxxxxxxxxxxx
pub fn generate_offline_uuid(username: &str) -> String {
    // Namespace for offline UUIDs (OfflinePlayer)
    const NAMESPACE: &[u8] = b"OfflinePlayer:";

    // Concatenate namespace and username
    let mut data = Vec::with_capacity(NAMESPACE.len() + username.len());
    data.extend_from_slice(NAMESPACE);
    data.extend_from_slice(username.as_bytes());

    // Calculate SHA1 hash using lighty-core
    let hash = lighty_core::calculate_sha1_bytes_raw(&data);

    // Format as UUID v5 (SHA1-based)
    // Version bits: 0101 (5) in the 13th position
    // Variant bits: 10xx in the 17th position (RFC 4122)
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-5{:01x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        hash[0], hash[1], hash[2], hash[3],
        hash[4], hash[5],
        hash[6] & 0x0f, hash[7],
        (hash[8] & 0x3f) | 0x80, hash[9],
        hash[10], hash[11], hash[12], hash[13], hash[14], hash[15]
    )
}

impl Clone for UserProfile {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            username: self.username.clone(),
            uuid: self.uuid.clone(),
            access_token: self.access_token.clone(),
            refresh_token: self.refresh_token.clone(),
            email: self.email.clone(),
            email_verified: self.email_verified,
            money: self.money,
            role: self.role.clone(),
            banned: self.banned,
            provider: self.provider.clone(),
            refresh_impl: self.refresh_impl.clone(),
            expires_in: self.expires_in,
            emited_at: self.emited_at,
        }
    }
}
