// Copyright (c) 2025 Hamadi
// Licensed under the MIT License

//! Java Runtime Execution
//!
//! This module provides a wrapper for executing Java processes with proper
//! I/O handling and lifecycle management.

use crate::errors::{JavaRuntimeError, JavaRuntimeResult};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::oneshot::Receiver;

/// Wrapper around a Java binary path for process execution
pub struct JavaRuntime(pub PathBuf);

impl JavaRuntime {
    /// Creates a new JavaRuntime from a binary path
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Spawns a Java process with the given arguments
    ///
    /// # Arguments
    /// * `arguments` - Command-line arguments for the Java process
    /// * `game_dir` - Working directory for the process
    ///
    /// # Returns
    /// A handle to the spawned child process
    ///
    /// # Errors
    /// Returns an error if the binary doesn't exist or the spawn fails
    pub async fn execute(&self, arguments: Vec<String>, game_dir: &Path) -> JavaRuntimeResult<Child> {
        // Validate binary exists
        if !self.0.exists() {
            return Err(JavaRuntimeError::NotFound {
                path: self.0.clone(),
            });
        }

        lighty_core::trace_debug!("Spawning Java process: {:?}", &self.0);

        // Build command
        let mut command = Command::new(&self.0);
        command
            .current_dir(game_dir)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // On Windows, hide the console window
        #[cfg(windows)]
        {
            // use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        // Spawn the process
        let child = command.spawn()?;

        lighty_core::trace_info!("Java process spawned successfully");
        Ok(child)
    }

    /// Streams stdout/stderr from the process with custom handlers
    ///
    /// This method handles I/O from the Java process, calling provided callbacks
    /// for stdout and stderr output. It continues until the process exits or
    /// the terminator signal is received.
    ///
    /// # Arguments
    /// * `process` - Mutable reference to the child process
    /// * `on_stdout` - Callback for stdout data
    /// * `on_stderr` - Callback for stderr data
    /// * `terminator` - Channel to signal early termination
    /// * `data` - User data passed to callbacks
    ///
    /// # Returns
    /// Ok(()) on clean exit, or error if the process exits with non-zero code
    ///
    /// # Note
    /// Exit code -1073740791 (Windows forceful termination) is not treated as an error
    pub async fn handle_io<D: Send + Sync>(
        &self,
        process: &mut Child,
        on_stdout: fn(&D, &[u8]) -> JavaRuntimeResult<()>,
        on_stderr: fn(&D, &[u8]) -> JavaRuntimeResult<()>,
        terminator: Receiver<()>,
        data: &D,
    ) -> JavaRuntimeResult<()> {
        // Extract stdout and stderr pipes
        let mut stdout = process
            .stdout
            .take()
            .ok_or(JavaRuntimeError::IoCaptureFailure)?;
        let mut stderr = process
            .stderr
            .take()
            .ok_or(JavaRuntimeError::IoCaptureFailure)?;

        // Prepare read buffers (stack-allocated for better performance)
        // 8KB is optimal for most Java logs while avoiding stack overflow
        let mut stdout_buffer = [0u8; 8192];
        let mut stderr_buffer = [0u8; 8192];

        tokio::pin!(terminator);

        // Main I/O loop
        loop {
            tokio::select! {
                // Handle stdout data
                result = stdout.read(&mut stdout_buffer) => {
                    match result {
                        Ok(bytes_read) if bytes_read > 0 => {
                            let _ = on_stdout(data, &stdout_buffer[..bytes_read]);
                        }
                        Ok(_) => {}, // EOF reached
                        Err(_) => break, // Stream closed
                    }
                },

                // Handle stderr data
                result = stderr.read(&mut stderr_buffer) => {
                    match result {
                        Ok(bytes_read) if bytes_read > 0 => {
                            let _ = on_stderr(data, &stderr_buffer[..bytes_read]);
                        }
                        Ok(_) => {}, // EOF reached
                        Err(_) => break, // Stream closed
                    }
                },

                // Handle early termination signal
                _ = &mut terminator => {
                    lighty_core::trace_debug!("Termination signal received, killing process");
                    process.kill().await?;
                    break;
                },

                // Handle process exit
                exit_result = process.wait() => {
                    let exit_status = exit_result?;
                    let exit_code = exit_status.code().unwrap_or(7900);

                    lighty_core::trace_debug!("Java process exited with code: {}", exit_code);

                    // Check for error exit codes
                    // -1073740791 = Windows forceful termination (not an error)
                    if exit_code != 0 && exit_code != -1073740791 {
                        return Err(JavaRuntimeError::NonZeroExit { code: exit_code });
                    }

                    break;
                },
            }
        }

        Ok(())
    }
}
