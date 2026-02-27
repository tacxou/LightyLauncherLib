use lighty_auth::offline::OfflineAuth;
use lighty_auth::Authenticator;

#[tokio::main]
async fn main() {
    // Création d'un authentificateur offline
    let mut auth = OfflineAuth::new("TestUser");
    let profile = auth.authenticate().await.expect("auth ok");
    println!("Avant refresh: {}", profile.uuid);

    // Appel du refresh via le champ refresh_impl (polymorphisme dynamique)
    let refreshed = profile.refresh_impl.as_ref().expect("refresh_impl absent").refresh_access_token(&profile).await.expect("refresh ok");
    println!("Après refresh: {}", refreshed.uuid);

    // Vérification que le refresh retourne bien un profil identique
    assert_eq!(profile.uuid, refreshed.uuid);
    println!("Token refresh OK (offline)");
}
