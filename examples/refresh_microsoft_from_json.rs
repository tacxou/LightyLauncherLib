use lighty_auth::{UserProfile, microsoft::MicrosoftAuth, Authenticator, AuthProvider};
use std::{fs, thread::sleep};


#[tokio::main]
async fn main() {
    // Tente de charger un UserProfile Microsoft depuis un fichier JSON de test
    let json_path = "examples/test_microsoft_profile.json";
    let json = fs::read_to_string(json_path).unwrap_or_default();
    let profile: Option<UserProfile> = if json.trim().is_empty() {
        None
    } else {
        serde_json::from_str::<UserProfile>(&json).ok().map(|mut s| {
            // refresh_impl n'est pas sérialisé, il faut le réinitialiser
            s.refresh_impl = match &s.provider {
                AuthProvider::Microsoft { .. } => Some(std::sync::Arc::new(lighty_auth::microsoft::MicrosoftRefresh)),
                AuthProvider::Azuriom { .. } => Some(std::sync::Arc::new(lighty_auth::azuriom::AzuriomRefresh)),
                AuthProvider::Offline => Some(std::sync::Arc::new(lighty_auth::offline::OfflineRefresh)),
                AuthProvider::Custom { .. } => unimplemented!("CustomRefresh n'est pas implémenté dans cet exemple"),
            };
            s
        })
    };

    let profile = if let Some(profile) = profile {
        println!("Profil chargé depuis le JSON");
        profile
    } else {
        println!("Aucun profil Microsoft trouvé, lancement de l'authentification...");
        let mut auth = MicrosoftAuth::new("7347d7b7-f14d-40c4-af19-f82204a7851e");
        auth.set_device_code_callback(|code, url| {
            println!("Please visit: {}", url);
            println!("And enter code: {}", code);
        });
        // Ici, il faudrait setter un vrai callback pour afficher le code à l'utilisateur
        let profile = auth.authenticate().await.map_err(|e| {
        let msg = format!("Auth failed: {:?}", e);
            println!("{}", msg);
            msg
        }).expect("Authentication should succeed");
        // Sauvegarde le profil pour la prochaine fois
        let json = serde_json::to_string_pretty(&profile).expect("serialize");
        fs::write(json_path, json).expect("write json");
        profile
    };

    println!("Avant refresh : access_token={:?} expires_in={} emited_at={:?}", profile.access_token, profile.expires_in, profile.emited_at);

    sleep(std::time::Duration::from_millis(2_000));

    // Appel du refresh dynamique
    let refreshed = profile.refresh_impl.as_ref().expect("refresh_impl absent").refresh_access_token(&profile).await.expect("refresh ok");
    println!("Après refresh : access_token={:?} expires_in={} emited_at={:?}", refreshed.access_token, refreshed.expires_in, refreshed.emited_at);
}
