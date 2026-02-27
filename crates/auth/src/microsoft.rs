// Copyright (c) 2025 Hamadi
// Licensed under the MIT License

// Microsoft OAuth 2.0 authentication for Minecraft
//
// Implements the Device Code Flow for authenticating Minecraft accounts via Microsoft.
// This is a multi-step process:
// 1. Request a device code
// 2. User authorizes via browser
// 3. Poll for token
// 4. Exchange for Xbox Live token
// 5. Exchange for XSTS token
// 6. Exchange for Minecraft token
// 7. Fetch Minecraft profile

use crate::Authenticator;
pub(crate) use lighty_core::hosts::HTTP_CLIENT as CLIENT;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(feature = "events")]
use lighty_event::{EventBus, Event, AuthEvent};

pub(crate) const MS_AUTH_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// Microsoft authenticator using Device Code Flow
///
/// This authentication method is suitable for CLI applications and launchers.
/// The user will need to visit a URL and enter a code to authorize.
///
/// # Example
/// ```no_run
/// use lighty_auth::microsoft::MicrosoftAuth;
/// use lighty_auth::Authenticator;
///
/// #[tokio::main]
/// async fn main() {
///     let mut auth = MicrosoftAuth::new("your-client-id");
///
///     // Set a callback to display the device code to the user
///     auth.set_device_code_callback(|code, url| {
///         println!("Please visit: {}", url);
///         println!("And enter code: {}", code);
///     });
///
///     let profile = auth.authenticate().await.unwrap();
///     println!("Logged in as: {}", profile.username);
/// }
/// ```
pub struct MicrosoftAuth {
    client_id: String,
    device_code_callback: Option<Box<dyn Fn(&str, &str) + Send + Sync>>,
    poll_interval: Duration,
    timeout: Duration,
}

impl MicrosoftAuth {
    /// Create a new Microsoft authenticator
    ///
    /// # Arguments
    /// - `client_id`: Your Azure AD application client ID
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            device_code_callback: None,
            poll_interval: Duration::from_secs(5),
            timeout: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Set a callback to display the device code to the user
    ///
    /// # Arguments
    /// - `callback`: Function that receives (code, verification_url)
    pub fn set_device_code_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str, &str) + Send + Sync + 'static,
    {
        self.device_code_callback = Some(Box::new(callback));
    }

    /// Set the polling interval
    ///
    /// Default: 5 seconds
    pub fn set_poll_interval(&mut self, interval: Duration) {
        self.poll_interval = interval;
    }

    /// Set the authentication timeout
    ///
    /// Default: 5 minutes
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Step 1: Request device code
    async fn request_device_code(&self) -> AuthResult<DeviceCodeResponse> {
        lighty_core::trace_debug!("Requesting device code");

        let response = CLIENT
            .post(&format!("{}/devicecode", MS_AUTH_URL))
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("scope", "XboxLive.signin offline_access"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            lighty_core::trace_error!(error = %error_text, "Failed to request device code");
            return Err(AuthError::InvalidResponse(error_text));
        }

        let device_code: DeviceCodeResponse = response.json().await?;
        lighty_core::trace_info!(user_code = %device_code.user_code, "Device code obtained");

        Ok(device_code)
    }

    /// Step 2: Poll for Microsoft token
    async fn poll_for_token(&self, device_code: &str) -> AuthResult<MicrosoftTokenResponse> {
        lighty_core::trace_debug!("Polling for Microsoft token");

        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > self.timeout {
                lighty_core::trace_error!("Device code expired");
                return Err(AuthError::DeviceCodeExpired);
            }

            sleep(self.poll_interval).await;

            let response = CLIENT
                .post(&format!("{}/token", MS_AUTH_URL))
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("client_id", &self.client_id),
                    ("device_code", device_code),
                ])
                .send()
                .await?;

            if response.status().is_success() {
                let token: MicrosoftTokenResponse = response.json().await?;
                lighty_core::trace_info!("Microsoft token obtained");
                return Ok(token);
            }

            let error: OAuthError = response.json().await?;

            match error.error.as_str() {
                "authorization_pending" => {
                    lighty_core::trace_debug!("Authorization pending, continuing to poll");
                    continue;
                }
                "authorization_declined" => {
                    lighty_core::trace_error!("User declined authorization");
                    return Err(AuthError::Cancelled);
                }
                "expired_token" => {
                    lighty_core::trace_error!("Device code expired");
                    return Err(AuthError::DeviceCodeExpired);
                }
                _ => {
                    lighty_core::trace_error!(error = %error.error, description = ?error.error_description, "OAuth error");
                    return Err(AuthError::Custom(error.error));
                }
            }
        }
    }

    /// Step 3: Exchange Microsoft token for Xbox Live token
    async fn get_xbox_token(&self, ms_token: &str) -> AuthResult<XboxTokenResponse> {
        lighty_core::trace_debug!("Requesting Xbox Live token");

        let response = CLIENT
            .post(XBOX_AUTH_URL)
            .json(&serde_json::json!({
                "Properties": {
                    "AuthMethod": "RPS",
                    "SiteName": "user.auth.xboxlive.com",
                    "RpsTicket": format!("d={}", ms_token)
                },
                "RelyingParty": "http://auth.xboxlive.com",
                "TokenType": "JWT"
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            lighty_core::trace_error!(error = %error_text, "Failed to get Xbox Live token");
            return Err(AuthError::InvalidResponse(error_text));
        }

        let xbox_token: XboxTokenResponse = response.json().await?;
        lighty_core::trace_info!("Xbox Live token obtained");

        Ok(xbox_token)
    }

    /// Step 4: Exchange Xbox Live token for XSTS token
    async fn get_xsts_token(&self, xbox_token: &str) -> AuthResult<XboxTokenResponse> {
        lighty_core::trace_debug!("Requesting XSTS token");

        let response = CLIENT
            .post(XSTS_AUTH_URL)
            .json(&serde_json::json!({
                "Properties": {
                    "SandboxId": "RETAIL",
                    "UserTokens": [xbox_token]
                },
                "RelyingParty": "rp://api.minecraftservices.com/",
                "TokenType": "JWT"
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let _status = response.status();
            let error_text = response.text().await?;

            // Check for specific error codes
            if error_text.contains("2148916233") {
                lighty_core::trace_error!("Account doesn't own Minecraft");
                return Err(AuthError::Custom("This Microsoft account doesn't own Minecraft".into()));
            }
            if error_text.contains("2148916238") {
                lighty_core::trace_error!("Account is from a country where Xbox Live is unavailable");
                return Err(AuthError::Custom("Xbox Live is not available in your country".into()));
            }

            lighty_core::trace_error!(status = %_status, error = %error_text, "Failed to get XSTS token");
            return Err(AuthError::InvalidResponse(error_text));
        }

        let xsts_token: XboxTokenResponse = response.json().await?;
        lighty_core::trace_info!("XSTS token obtained");

        Ok(xsts_token)
    }

    /// Step 5: Exchange XSTS token for Minecraft token
    async fn get_minecraft_token(&self, xsts_token: &str, uhs: &str) -> AuthResult<MinecraftTokenResponse> {
        lighty_core::trace_debug!("Requesting Minecraft token");

        let response = CLIENT
            .post(MC_AUTH_URL)
            .json(&serde_json::json!({
                "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token)
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            lighty_core::trace_error!(error = %error_text, "Failed to get Minecraft token");
            return Err(AuthError::InvalidResponse(error_text));
        }

        let mc_token: MinecraftTokenResponse = response.json().await?;
        lighty_core::trace_info!("Minecraft token obtained");

        Ok(mc_token)
    }

    /// Step 6: Fetch Minecraft profile
    async fn get_minecraft_profile(&self, mc_token: &str) -> AuthResult<MinecraftProfile> {
        lighty_core::trace_debug!("Fetching Minecraft profile");

        let response = CLIENT
            .get(MC_PROFILE_URL)
            .header("Authorization", format!("Bearer {}", mc_token))
            .send()
            .await?;

        if !response.status().is_success() {
            let _status = response.status();
            let error_text = response.text().await?;
            lighty_core::trace_error!(status = %_status, error = %error_text, "Failed to get Minecraft profile");
            return Err(AuthError::InvalidResponse(error_text));
        }

        let profile: MinecraftProfile = response.json().await?;
        lighty_core::trace_info!(username = %profile.name, uuid = %profile.id, "Minecraft profile obtained");

        Ok(profile)
    }
}

impl Authenticator for MicrosoftAuth {
    async fn authenticate(
        &mut self,
        #[cfg(feature = "events")] event_bus: Option<&EventBus>,
    ) -> AuthResult<UserProfile> {
        // Emit authentication started
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationStarted {
                provider: "Microsoft".to_string(),
            }));
        }

        // Step 1: Request device code
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationInProgress {
                provider: "Microsoft".to_string(),
                step: "Requesting device code".to_string(),
            }));
        }

        let device_code_response = match self.request_device_code().await {
            Ok(response) => response,
            Err(e) => {
                #[cfg(feature = "events")]
                if let Some(bus) = event_bus {
                    bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                        provider: "Microsoft".to_string(),
                        error: format!("Failed to request device code: {}", e),
                    }));
                }
                return Err(e);
            }
        };

        // Notify user via callback
        if let Some(callback) = &self.device_code_callback {
            callback(&device_code_response.user_code, &device_code_response.verification_uri);
        } else {
            lighty_core::trace_warn!("No device code callback set - user won't see the authorization URL");
        }

        // Step 2: Poll for Microsoft token
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationInProgress {
                provider: "Microsoft".to_string(),
                step: "Waiting for user authorization".to_string(),
            }));
        }

        let ms_token = match self.poll_for_token(&device_code_response.device_code).await {
            Ok(token) => token,
            Err(e) => {
                #[cfg(feature = "events")]
                if let Some(bus) = event_bus {
                    bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                        provider: "Microsoft".to_string(),
                        error: format!("Failed to get Microsoft token: {}", e),
                    }));
                }
                return Err(e);
            }
        };

        // Step 3: Get Xbox Live token
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationInProgress {
                provider: "Microsoft".to_string(),
                step: "Exchanging for Xbox Live token".to_string(),
            }));
        }

        let xbox_token = match self.get_xbox_token(&ms_token.access_token).await {
            Ok(token) => token,
            Err(e) => {
                #[cfg(feature = "events")]
                if let Some(bus) = event_bus {
                    bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                        provider: "Microsoft".to_string(),
                        error: format!("Failed to get Xbox Live token: {}", e),
                    }));
                }
                return Err(e);
            }
        };

        // Step 4: Get XSTS token
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationInProgress {
                provider: "Microsoft".to_string(),
                step: "Exchanging for XSTS token".to_string(),
            }));
        }

        let xsts_token = match self.get_xsts_token(&xbox_token.token).await {
            Ok(token) => token,
            Err(e) => {
                #[cfg(feature = "events")]
                if let Some(bus) = event_bus {
                    bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                        provider: "Microsoft".to_string(),
                        error: format!("Failed to get XSTS token: {}", e),
                    }));
                }
                return Err(e);
            }
        };

        // Extract UHS from XSTS token
        let uhs = xsts_token
            .display_claims
            .get("xui")
            .and_then(|xui| xui.get(0))
            .and_then(|user| user.get("uhs"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                let error = AuthError::InvalidResponse("Missing UHS in XSTS token".into());
                #[cfg(feature = "events")]
                if let Some(bus) = event_bus {
                    bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                        provider: "Microsoft".to_string(),
                        error: "Missing UHS in XSTS token".to_string(),
                    }));
                }
                error
            })?;

        // Step 5: Get Minecraft token
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationInProgress {
                provider: "Microsoft".to_string(),
                step: "Exchanging for Minecraft token".to_string(),
            }));
        }

        let mc_token = match self.get_minecraft_token(&xsts_token.token, uhs).await {
            Ok(token) => token,
            Err(e) => {
                #[cfg(feature = "events")]
                if let Some(bus) = event_bus {
                    bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                        provider: "Microsoft".to_string(),
                        error: format!("Failed to get Minecraft token: {}", e),
                    }));
                }
                return Err(e);
            }
        };

        // Step 6: Get Minecraft profile
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationInProgress {
                provider: "Microsoft".to_string(),
                step: "Fetching Minecraft profile".to_string(),
            }));
        }

        let mc_profile = match self.get_minecraft_profile(&mc_token.access_token).await {
            Ok(profile) => profile,
            Err(e) => {
                #[cfg(feature = "events")]
                if let Some(bus) = event_bus {
                    bus.emit(Event::Auth(AuthEvent::AuthenticationFailed {
                        provider: "Microsoft".to_string(),
                        error: format!("Failed to get Minecraft profile: {}", e),
                    }));
                }
                return Err(e);
            }
        };

        // Format UUID with dashes
        let uuid = format_uuid(&mc_profile.id);

        // Emit authentication success
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            bus.emit(Event::Auth(AuthEvent::AuthenticationSuccess {
                provider: "Microsoft".to_string(),
                username: mc_profile.name.clone(),
                uuid: uuid.clone(),
            }));
        }

        // Ajout de emited_at avec la date de la requête
        Ok(UserProfile {
            provider: crate::AuthProvider::Microsoft { client_id: self.client_id.clone() },
            refresh_impl: Some(std::sync::Arc::new(MicrosoftRefresh)),
            id: None,
            username: mc_profile.name,
            uuid,
            access_token: Some(mc_token.access_token),
            refresh_token: ms_token.refresh_token,
            email: None,
            email_verified: true,
            money: None,
            role: None,
            banned: false,
            expires_in: ms_token.expires_in,
            emited_at: Some(chrono::Utc::now()),
        })
    }
}

/// Format UUID string with dashes
fn format_uuid(uuid: &str) -> String {
    if uuid.len() != 32 {
        return uuid.to_string();
    }

    format!(
        "{}-{}-{}-{}-{}",
        &uuid[0..8],
        &uuid[8..12],
        &uuid[12..16],
        &uuid[16..20],
        &uuid[20..32]
    )
}

// Response structures

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct MicrosoftTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct XboxTokenResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MinecraftTokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OAuthError {
    error: String,
    error_description: Option<String>,
}

use crate::auth::{UserProfile, TokenRefreshable, AuthResult};

pub struct MicrosoftRefresh;

#[async_trait::async_trait]
impl TokenRefreshable for MicrosoftRefresh {
    async fn refresh_access_token(&self, profile: &UserProfile) -> crate::AuthResult<UserProfile> {
        let client_id = match &profile.provider {
            crate::AuthProvider::Microsoft { client_id } => client_id,
            _ => return Err(crate::AuthError::Custom("Not a Microsoft profile".into())),
        };
        refresh_microsoft_access_token(profile, client_id).await
    }
}

#[async_trait::async_trait]
pub trait TokenRefreshableProvider: Send + Sync {
    async fn refresh_access_token(&self, profile: &UserProfile) -> AuthResult<UserProfile>;
}

pub struct MicrosoftRefreshProvider;

#[async_trait::async_trait]
impl TokenRefreshableProvider for MicrosoftRefreshProvider {
    async fn refresh_access_token(&self, profile: &UserProfile) -> AuthResult<UserProfile> {
        let client_id = match &profile.provider {
            crate::AuthProvider::Microsoft { client_id } => client_id,
            _ => return Err(crate::AuthError::Custom("Not a Microsoft profile".into())),
        };
        refresh_microsoft_access_token(profile, client_id).await
    }
}
use crate::AuthError;
/// Rafraîchit le token Microsoft à partir d'un UserProfile
pub async fn refresh_microsoft_access_token(profile: &UserProfile, client_id: &str) -> AuthResult<UserProfile> {
    use crate::microsoft::{CLIENT, MS_AUTH_URL};
    if let Some(refresh_token) = &profile.refresh_token {
        let response = CLIENT
            .post(&format!("{}/token", MS_AUTH_URL))
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", client_id),
                ("refresh_token", refresh_token),
                ("scope", "XboxLive.signin offline_access"),
            ])
            .send()
            .await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AuthError::InvalidResponse(error_text));
        }
        let token: crate::microsoft::MicrosoftTokenResponse = response.json().await?;
        let mut new_profile = profile.clone();
        new_profile.access_token = Some(token.access_token);
        new_profile.refresh_token = token.refresh_token;
        // Met à jour la date d'émission et le temps d'expiration
        new_profile.emited_at = Some(chrono::Utc::now());
        new_profile.expires_in = token.expires_in;
        Ok(new_profile)
    } else {
        Err(AuthError::Custom("No refresh_token available".into()))
    }
}