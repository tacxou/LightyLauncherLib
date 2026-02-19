use lighty_launcher::prelude::*;

const QUALIFIER: &str = "com";
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

    // Configure le downloader (optionnel - valeurs par défaut si non appelé)
    init_downloader_config(
        DownloaderConfig {
        max_concurrent_downloads: 100,   // Plus de downloads concurrents car de base 50
        max_retries: 5,                  // Plus de tentatives car de base 3
        initial_delay_ms: 50,            // Délai initial plus long car de base 20
    }
    );
    
    let mut auth = MicrosoftAuth::new("7347d7b7-f14d-40c4-af19-f82204a7851e");

    // Display device code to user
    auth.set_device_code_callback(|code, url| {
        println!("Please visit: {}", url);
        println!("And enter code: {}", code);
    });

    #[cfg(feature = "events")]
    let profile = auth.authenticate(None).await?;

    #[cfg(not(feature = "events"))]
    let profile = auth.authenticate().await?;

    // Authenticate
    // let mut auth = OfflineAuth::new("Hamadi");
    // #[cfg(feature = "events")]
    // let profile = auth.authenticate(None).await?;
    // #[cfg(not(feature = "events"))]
    // let profile = auth.authenticate().await?;

    let mut version = VersionBuilder::new("vanilla-1.7.10", Loader::Vanilla, "", "1.7.10", launcher_dir);

    version.launch(&profile, JavaDistribution::Temurin)
        .with_jvm_options()
            .set("Xmx", "4G")
            .set("Xms", "2G")
            .done()
        .with_arguments()
            .set("width", "1920")
            .set("height", "1080")
            .done()
        .run()
        .await?;

    trace_info!("Launch successful!");

    Ok(())
}
