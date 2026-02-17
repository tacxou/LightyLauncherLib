//NOT FINISHED - NeoForge implementation is still in progress
use lighty_launcher::prelude::*;

const QUALIFIER: &str = "fr";
const ORGANIZATION: &str = ".LightyLauncher";
const APPLICATION: &str = "";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let _app_state = AppState::new(
        QUALIFIER.to_string(),
        ORGANIZATION.to_string(),
        APPLICATION.to_string(),
    )?;

    let launcher_dir = AppState::get_project_dirs();

    // Authenticate
    let mut auth = OfflineAuth::new("Hamadi");
    #[cfg(feature = "events")]
    let profile = auth.authenticate(None).await?;
    #[cfg(not(feature = "events"))]
    let profile = auth.authenticate().await?;

    // Build and launch NeoForge instance
    let mut neoforge = VersionBuilder::new(
        "neoforge_before_1-20",
        Loader::NeoForge,
        "47.1.106",
        "1.20.1",
        launcher_dir,
    );

    neoforge
        .launch(&profile, JavaDistribution::Temurin)
        .run()
        .await?;

    trace_info!("NeoForge launch successful!");

    Ok(())
}
