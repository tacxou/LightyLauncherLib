// Copyright (c) 2025 Hamadi
// Licensed under the MIT License

//! Azuriom CMS authentication
//!
//! Supports authentication via the Azuriom API with:
//! - Email/password login
//! - Two-factor authentication (2FA)
//! - Token verification
//! - Logout

use crate::{Authenticator, AuthError, AuthResult, UserProfile, UserRole};
use lighty_core::hosts::HTTP_CLIENT as CLIENT;
use serde::Deserialize;

#[cfg(feature = "events")]
use lighty_event::{EventBus, Event, AuthEvent};

/// Azuriom authenticator
///
/// Authenticates users via an Azuriom CMS instance.
///
/// # Example
/// ```no_run
/// use lighty_auth::azuriom::AzuriomAuth;
/// use lighty_auth::Authenticator;
///
/// #[tokio::main]
/// async fn main() {
///     let mut auth = AzuriomAuth::new(
///         "https://example.com",
///         "user@example.com",
///         "password123"
///     );
///
///     let profile = auth.authenticate().await.unwrap();
///     println!("Logged in as: {}", profile.username);
/// }
/// ```
pub struct AzuriomAuth {
    base_url: String,
    email: String,
    password: String,
    two_factor_code: Option<String>,
}

impl AzuriomAuth {
    /// Create a new Azuriom authenticator
    ///
    /// # Arguments
    /// - `base_url`: Base URL of the Azuriom instance (e.g., "https://example.com")
    /// - `email`: User email address
    /// - `password`: User password
    pub fn new(base_url: impl Into<String>, email: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            email: email.into(),
            password: password.into(),
            two_factor_code: None,
        }
    }

    /// Set the 2FA code (call this if authentication returns `TwoFactorRequired`)
    ///
    /// # Arguments
    /// - `code`: The 2FA code from the authenticator app
    pub fn set_two_factor_code(&mut self, code: impl Into<String>) {
        self.two_factor_code = Some(code.into());
    }

    /// Clear the 2FA code
    pub fn clear_two_factor_code(&mut self) {
        self.two_factor_code = None;
    }
}


/// Azuriom API response for successful authentication
#[derive(Debug, Deserialize)]
struct AzuriomAuthResponse {
    id: u64,
    username: String,
    uuid: String,
    access_token: String,
    email_verified: Option<bool>,
    money: Option<f64>,
    role: Option<AzuriomRole>,
    banned: Option<bool>,
}

/// Azuriom role information
#[derive(Debug, Deserialize)]
struct AzuriomRole {
    name: String,
    color: Option<String>,
}

#[allow(dead_code)]
/// Azuriom API error response
#[derive(Debug, Deserialize)]
struct AzuriomErrorResponse {
    status: String, // Always "error"
    reason: String,
    message: String,
}
impl Authenticator for AzuriomAuth {
    async fn authenticate(
        &mut self,
        #[cfg(feature = "events")] event_bus: Option<&EventBus>,
    ) -> AuthResult<UserProfile> {
        let url = format!("{}/api/auth/authenticate", self.base_url);
        lighty_core::trace_debug!(url = %url, email = %self.email, "Authenticating with Azuriom");

        // Emit authentication started
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationStarted {
                provider: "Azuriom".to_string(),
            }));
        }

        // Build request body
        let mut body = serde_json::json!({
            "email": self.email,
            "password": self.password,
        });

        // Add 2FA code if provided
        if let Some(code) = &self.two_factor_code {
            body["code"] = serde_json::json!(code);
        }

        // Send request
        let response = CLIENT
            .post(&url)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        // Parse response
        if status.is_success() {
            let azuriom_response: AzuriomAuthResponse = serde_json::from_str(&response_text)
                .map_err(|e| AuthError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

            // Check if account is banned
            if azuriom_response.banned.unwrap_or(false) {
                lighty_core::trace_error!(username = %azuriom_response.username, "Account is banned");
                #[cfg(feature = "events")]
                if let Some(bus) = event_bus {
                    bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                        provider: "Azuriom".to_string(),
                        error: "Account is banned".to_string(),
                    }));
                }
                return Err(AuthError::AccountBanned(
                    azuriom_response.username.clone()
                ));
            }

            lighty_core::trace_info!(username = %azuriom_response.username, uuid = %azuriom_response.uuid, "Successfully authenticated");

            // Emit authentication success
            #[cfg(feature = "events")]
            if let Some(bus) = event_bus {
                bus.emit(Event::Auth(AuthEvent::AuthenticationSuccess {
                    provider: "Azuriom".to_string(),
                    username: azuriom_response.username.clone(),
                    uuid: azuriom_response.uuid.clone(),
                }));
            }

            Ok(UserProfile {
                id: Some(azuriom_response.id),
                username: azuriom_response.username,
                uuid: azuriom_response.uuid,
                access_token: Some(azuriom_response.access_token),
                email: Some(self.email.clone()),
                email_verified: azuriom_response.email_verified.unwrap_or(true),
                money: azuriom_response.money,
                role: azuriom_response.role.map(|r| UserRole {
                    name: r.name,
                    color: r.color,
                }),
                banned: azuriom_response.banned.unwrap_or(false),
            })
        } else {
            // Parse error response
            let error_response: AzuriomErrorResponse = serde_json::from_str(&response_text)
                .map_err(|_| AuthError::InvalidResponse(format!("HTTP {}: {}", status, response_text)))?;

            lighty_core::trace_error!(reason = %error_response.reason, message = %error_response.message, "Authentication failed");

            let error = match error_response.reason.as_str() {
                "invalid_credentials" => AuthError::InvalidCredentials,
                "requires_2fa" => AuthError::TwoFactorRequired,
                "invalid_2fa" => AuthError::Invalid2FACode,
                "email_not_verified" => AuthError::EmailNotVerified,
                "banned" => AuthError::AccountBanned(String::new()),
                _ => AuthError::Custom(error_response.message.clone()),
            };

            // Emit authentication failed
            #[cfg(feature = "events")]
            if let Some(bus) = event_bus {
                bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                    provider: "Azuriom".to_string(),
                    error: error_response.message,
                }));
            }

            Err(error)
        }
    }

    async fn verify(&self, token: &str) -> AuthResult<UserProfile> {
        let url = format!("{}/api/auth/verify", self.base_url);
        lighty_core::trace_debug!(url = %url, "Verifying token");

        let response = CLIENT
            .post(&url)
            .json(&serde_json::json!({
                "access_token": token
            }))
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        if status.is_success() {
            let azuriom_response: AzuriomAuthResponse = serde_json::from_str(&response_text)
                .map_err(|e| AuthError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

            lighty_core::trace_info!(username = %azuriom_response.username, "Token verified successfully");

            Ok(UserProfile {
                id: Some(azuriom_response.id),
                username: azuriom_response.username,
                uuid: azuriom_response.uuid,
                access_token: Some(azuriom_response.access_token),
                email: None, // Not returned by verify endpoint
                email_verified: azuriom_response.email_verified.unwrap_or(true),
                money: azuriom_response.money,
                role: azuriom_response.role.map(|r| UserRole {
                    name: r.name,
                    color: r.color,
                }),
                banned: azuriom_response.banned.unwrap_or(false),
            })
        } else {
            lighty_core::trace_error!(status = %status, "Token verification failed");
            Err(AuthError::InvalidToken)
        }
    }

    async fn logout(&self, token: &str) -> AuthResult<()> {
        let url = format!("{}/api/auth/logout", self.base_url);
        lighty_core::trace_debug!(url = %url, "Logging out");

        let response = CLIENT
            .post(&url)
            .json(&serde_json::json!({
                "access_token": token
            }))
            .send()
            .await?;

        if response.status().is_success() {
            lighty_core::trace_info!("Successfully logged out");
            Ok(())
        } else {
            lighty_core::trace_error!(status = %response.status(), "Logout failed");
            Err(AuthError::InvalidToken)
        }
    }
}


