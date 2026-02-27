// Copyright (c) 2025 Hamadi
// Licensed under the MIT License

// Offline authentication - no network required
//
// Generates a deterministic UUID v5 (SHA1-based) from the username.
// No token validation or verification.

use std::sync::Arc;
use crate::{Authenticator, AuthError, AuthResult, UserProfile, generate_offline_uuid, auth::TokenRefreshable};

#[cfg(feature = "events")]
use lighty_event::{EventBus, Event, AuthEvent};

/// Offline authenticator
///
/// Generates a deterministic UUID from the username without any network calls.
/// Suitable for offline play or testing.
///
/// # Example
/// ```no_run
/// use lighty_auth::offline::OfflineAuth;
/// use lighty_auth::Authenticator;
///
/// #[tokio::main]
/// async fn main() {
///     let mut auth = OfflineAuth::new("Player123");
///     let profile = auth.authenticate().await.unwrap();
///     println!("UUID: {}", profile.uuid);
/// }
/// ```
pub struct OfflineAuth {
    username: String,
}

impl OfflineAuth {
    /// Create a new offline authenticator
    ///
    /// # Arguments
    /// - `username`: The username to authenticate with
    ///
    /// # Returns
    /// A new `OfflineAuth` instance
    pub fn new(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
        }
    }

    /// Get the username
    pub fn username(&self) -> &str {
        &self.username
    }
}

impl Authenticator for OfflineAuth {
    async fn authenticate(
        &mut self,
        #[cfg(feature = "events")] event_bus: Option<&EventBus>,
    ) -> AuthResult<UserProfile> {
        // Emit authentication started
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationStarted {
                provider: "Offline".to_string(),
            }));
        }

        // Validate username
        if self.username.is_empty() {
            #[cfg(feature = "events")]
            if let Some(bus) = event_bus {
                bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                    provider: "Offline".to_string(),
                    error: "Username cannot be empty".to_string(),
                }));
            }
            return Err(AuthError::InvalidCredentials);
        }

        if self.username.len() < 3 || self.username.len() > 16 {
            let error_msg = "Username must be between 3 and 16 characters".to_string();
            #[cfg(feature = "events")]
            if let Some(bus) = event_bus {
                bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                    provider: "Offline".to_string(),
                    error: error_msg.clone(),
                }));
            }
            return Err(AuthError::Custom(error_msg));
        }

        // Check for valid characters (alphanumeric + underscore)
        if !self.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let error_msg = "Username can only contain letters, numbers, and underscores".to_string();
            #[cfg(feature = "events")]
            if let Some(bus) = event_bus {
                bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                    provider: "Offline".to_string(),
                    error: error_msg.clone(),
                }));
            }
            return Err(AuthError::Custom(error_msg));
        }

        // Generate deterministic UUID
        let uuid = generate_offline_uuid(&self.username);

        // Emit authentication success
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationSuccess {
                provider: "Offline".to_string(),
                username: self.username.clone(),
                uuid: uuid.clone(),
            }));
        }

            Ok(UserProfile {
                    provider: crate::AuthProvider::Offline,
                    refresh_impl: Some(Arc::new(OfflineRefresh)),
                id: None,
                username: self.username.clone(),
                uuid,
                access_token: None,
                refresh_token: None,
                email: None,
                email_verified: false,
                money: None,
                role: None,
                banned: false,
                expires_in: 0,
                emited_at: Some(chrono::Utc::now()),
            })
    }
}

pub struct OfflineRefresh;

#[async_trait::async_trait]
impl TokenRefreshable for OfflineRefresh {
    async fn refresh_access_token(&self, profile: &UserProfile) -> AuthResult<UserProfile> {
        Ok(profile.clone())
    }
}
