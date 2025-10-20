//! Background worker for file operations

use crossbeam_channel::{bounded, Receiver, Sender};
use flux_lib::archive::{self, PackOptions};
use std::path::PathBuf;
use std::thread;
use tracing::{error, info};

/// Worker command
#[derive(Debug)]
pub enum Command {
    /// Pack files
    Pack {
        inputs: Vec<PathBuf>,
        output: PathBuf,
        options: PackOptions,
    },
    /// Extract archive
    Extract {
        archive: PathBuf,
        output_dir: PathBuf,
    },
    /// Cancel current operation
    Cancel,
}

/// Worker event
#[derive(Debug, Clone)]
pub enum Event {
    /// Progress update
    Progress {
        current: u64,
        total: u64,
        message: String,
    },
    /// Log message
    Log(String),
    /// Operation completed
    Completed(String),
    /// Operation failed
    Failed(String),
}

/// Background worker
pub struct Worker {
    sender: Sender<Command>,
    receiver: Receiver<Event>,
}

impl Worker {
    /// Create a new worker
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = bounded(10);
        let (evt_tx, evt_rx) = bounded(100);
        
        // Spawn worker thread
        thread::spawn(move || {
            worker_thread(cmd_rx, evt_tx);
        });
        
        Self {
            sender: cmd_tx,
            receiver: evt_rx,
        }
    }
    
    /// Send a command to the worker
    pub fn send(&self, command: Command) -> Result<(), crossbeam_channel::SendError<Command>> {
        self.sender.send(command)
    }
    
    /// Try to receive an event
    pub fn try_recv(&self) -> Option<Event> {
        self.receiver.try_recv().ok()
    }
}

fn worker_thread(commands: Receiver<Command>, events: Sender<Event>) {
    info!("Worker thread started");
    
    while let Ok(command) = commands.recv() {
        match command {
            Command::Pack { inputs, output, options } => {
                info!("Packing {:?} to {:?}", inputs, output);
                
                let _ = events.send(Event::Progress {
                    current: 0,
                    total: 100,
                    message: "Starting pack operation...".to_string(),
                });
                
                // Handle single or multiple inputs
                let result = if inputs.len() == 1 {
                    archive::pack_with_strategy(&inputs[0], &output, None, options)
                } else {
                    // For multiple inputs, create a temporary directory and pack that
                    // This is a simplified approach - in production, we'd handle this better
                    error!("Multiple input packing not yet implemented in GUI");
                    Err(flux_lib::Error::UnsupportedOperation(
                        "Multiple input packing not yet implemented".to_string()
                    ))
                };
                
                match result {
                    Ok(()) => {
                        let _ = events.send(Event::Completed(
                            format!("Successfully packed to {:?}", output)
                        ));
                    }
                    Err(e) => {
                        let _ = events.send(Event::Failed(
                            format!("Pack failed: {}", e)
                        ));
                    }
                }
            }
            
            Command::Extract { archive, output_dir } => {
                info!("Extracting {:?} to {:?}", archive, output_dir);
                
                let _ = events.send(Event::Progress {
                    current: 0,
                    total: 100,
                    message: "Starting extract operation...".to_string(),
                });
                
                let result = archive::extract(&archive, &output_dir);
                
                match result {
                    Ok(()) => {
                        let _ = events.send(Event::Completed(
                            format!("Successfully extracted to {:?}", output_dir)
                        ));
                    }
                    Err(e) => {
                        let _ = events.send(Event::Failed(
                            format!("Extract failed: {}", e)
                        ));
                    }
                }
            }
            
            Command::Cancel => {
                info!("Cancel command received");
                let _ = events.send(Event::Log("Operation cancelled by user".to_string()));
                // In a real implementation, we would interrupt the ongoing operation
                // For now, we just log it
            }
        }
    }
    
    info!("Worker thread exiting");
}