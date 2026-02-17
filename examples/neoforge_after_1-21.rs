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
    
    #[cfg(feature = "events")]
    let event_bus = EventBus::new(1000);

    #[cfg(feature = "events")]
    let (instance_exit_tx, instance_exit_rx) = tokio::sync::oneshot::channel::<Option<i32>>();

    #[cfg(feature = "events")] {
        let mut receiver = event_bus.subscribe();
        let mut instance_exit_tx = Some(instance_exit_tx);

        tokio::spawn(async move {
            while let Ok(event) = receiver.next().await {
                match event {
                    Event::ConsoleOutput(e) => {
                        let prefix = match e.stream {
                            ConsoleStream::Stdout => "[GAME]",
                            ConsoleStream::Stderr => "[ERR]",
                        };
                        println!("{} {}", prefix, e.line);
                    }
                    Event::InstanceExited(e) => {
                        println!("\n⚠ Instance exited with code: {:?}", e.exit_code);
                        if let Some(tx) = instance_exit_tx.take() {
                            let _ = tx.send(e.exit_code);
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    // Authenticate
    let mut auth = OfflineAuth::new("Hamadi");
    #[cfg(feature = "events")]
    let profile = auth.authenticate(None).await?;
    #[cfg(not(feature = "events"))]
    let profile = auth.authenticate().await?;

    // Build and launch NeoForge instance
    let mut neoforge = VersionBuilder::new(
        "neoforge_after_1-21",
        Loader::NeoForge,
        "21.1.219",
        "1.21.1",
        launcher_dir,
    );

    #[cfg(feature = "events")]
    neoforge
        .launch(&profile, JavaDistribution::Temurin)
        .with_event_bus(&event_bus)
        .run()
        .await?;

    #[cfg(not(feature = "events"))]
    neoforge
        .launch(&profile, JavaDistribution::Temurin)
        .run()
        .await?;

    #[cfg(feature = "events")]
    let _ = instance_exit_rx.await;

    trace_info!("NeoForge launch successful!");

    Ok(())
}
