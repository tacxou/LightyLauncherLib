use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;

#[cfg(feature = "events")]
use lighty_event::EventBus;

/// Handle console streams (stdout/stderr) from a running game instance
///
/// This function spawns asynchronous tasks to:
/// - Read and emit stdout lines (Minecraft includes its own timestamps in the log text)
/// - Read and emit stderr lines
/// - Wait for the process to exit and emit exit event
/// - Unregister the instance when done
///
/// Note: Frontend should not display the event timestamp for stdout as Minecraft
/// already includes timestamps in its log format
pub(crate) async fn handle_console_streams(
    pid: u32,
    instance_name: String,
    mut child: Child,
    #[cfg(feature = "events")] event_bus: Option<EventBus>,
) {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Handler stdout
    if let Some(stdout) = stdout {
        #[allow(unused_variables)]
        let instance_name = instance_name.clone();
        #[cfg(feature = "events")]
        let event_bus_clone = event_bus.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(_line)) = lines.next_line().await {
                #[cfg(feature = "events")]
                {
                    use lighty_event::{ConsoleOutputEvent, ConsoleStream, Event};
                    use std::time::SystemTime;

                    if let Some(ref bus) = event_bus_clone {
                        bus.emit(Event::ConsoleOutput(ConsoleOutputEvent {
                            pid,
                            instance_name: instance_name.clone(),
                            stream: ConsoleStream::Stdout,
                            line: _line,
                            timestamp: SystemTime::now(),
                        }));
                    }
                }
            }
        });
    }

    // Handler stderr
    if let Some(stderr) = stderr {
        #[allow(unused_variables)]
        let instance_name = instance_name.clone();
        #[cfg(feature = "events")]
        let event_bus_clone = event_bus.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(_line)) = lines.next_line().await {
                #[cfg(feature = "events")]
                {
                    use lighty_event::{ConsoleOutputEvent, ConsoleStream, Event};
                    use std::time::SystemTime;

                    if let Some(ref bus) = event_bus_clone {
                        bus.emit(Event::ConsoleOutput(ConsoleOutputEvent {
                            pid,
                            instance_name: instance_name.clone(),
                            stream: ConsoleStream::Stderr,
                            line: _line,
                            timestamp: SystemTime::now(),
                        }));
                    }
                }
            }
        });
    }

    // Wait for process to exit
    match child.wait().await {
        Ok(_status) => {
            #[cfg(feature = "events")]
            {
                use lighty_event::{Event, InstanceExitedEvent};
                use std::time::SystemTime;

                if let Some(ref bus) = event_bus {
                    bus.emit(Event::InstanceExited(InstanceExitedEvent {
                        pid,
                        instance_name: instance_name.clone(),
                        exit_code: _status.code(),
                        timestamp: SystemTime::now(),
                    }));
                }
            }

            lighty_core::trace_info!(
                pid = pid,
                instance = %instance_name,
                exit_code = ?_status.code(),
                "Instance exited"
            );
        }
        Err(_e) => {
            lighty_core::trace_error!(
                pid = pid,
                instance = %instance_name,
                error = %_e,
                "Error waiting for instance"
            );
        }
    }

    // Cleanup
    use super::INSTANCE_MANAGER;
    let _ = INSTANCE_MANAGER.unregister_instance(pid).await;
}
