// Copyright (c) 2025 Hamadi
// Licensed under the MIT License

//! Authentication module for Lighty Launcher
//!
//! Provides multiple authentication providers and a trait-based system for custom implementations.
//!
//! ## Built-in Providers
//!
//! - **Offline**: No network authentication, generates deterministic UUIDs from username
//! - **Microsoft**: OAuth 2.0 authentication via Microsoft/Xbox Live
//! - **Azuriom**: Authentication via Azuriom CMS API
//!
//! ## Custom Authentication
//!
//! Implement the `Authenticator` trait to create your own authentication provider:
//!
//! ```rust
//! use lighty_auth::{Authenticator, UserProfile, UserRole, AuthResult, AuthError};
//! # use async_trait::async_trait;
//!
//! pub struct MyCustomAuth {
//!     api_url: String,
//!     username: String,
//! }
//!
//! impl MyCustomAuth {
//!     pub fn new(api_url: String, username: String) -> Self {
//!         Self { api_url, username }
//!     }
//! }
//!
//! impl Authenticator for MyCustomAuth {
//!     async fn authenticate(
//!         &mut self,
//!         #[cfg(feature = "events")] event_bus: Option<&lighty_event::EventBus>,
//!     ) -> AuthResult<UserProfile> {
//!         // Your custom authentication logic here
//!
//!         // Example: make HTTP request to your API
//!         // let response = reqwest::get(&self.api_url).await?;
//!         // let data = response.json::<YourResponse>().await?;
//!
//!         Ok(UserProfile {
//!             username: self.username.clone(),
//!             uuid: "your-uuid".to_string(),
//!             access_token: Some("your-token".to_string()),
//!             role: UserRole::User,
//!         })
//!     }
//! }
//! ```
//!
//! ## Helpers
//!
//! Use the `generate_offline_uuid()` function to create deterministic UUIDs:
//!
//! ```rust
//! use lighty_auth::generate_offline_uuid;
//!
//! let uuid = generate_offline_uuid("PlayerName");
//! println!("UUID: {}", uuid); // Always the same for this username
//! ```

pub mod auth;
pub use auth::TokenRefreshable;
mod errors;

pub mod offline;
pub mod azuriom;
pub mod microsoft;

// Re-export core types
pub use auth::{
    AuthProvider, AuthResult, Authenticator, UserProfile, UserRole, generate_offline_uuid,
};
pub use errors::AuthError;
