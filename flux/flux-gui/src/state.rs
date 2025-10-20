//! Application state management

use std::path::PathBuf;
use flux_lib::strategy::Algorithm;

/// Current operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Ready to accept input
    Idle,
    /// Packing files
    Pack,
    /// Extracting files
    Extract,
}

/// Operation status
#[derive(Debug, Clone)]
pub enum Status {
    /// No operation in progress
    Ready,
    /// Operation in progress
    Working {
        progress: f32,
        message: String,
    },
    /// Operation completed successfully
    Success(String),
    /// Operation failed
    Error(String),
}

/// Compression settings
#[derive(Debug, Clone)]
pub struct CompressionSettings {
    pub algorithm: Algorithm,
    pub level: u32,
    pub threads: usize,
    pub smart: bool,
    pub follow_symlinks: bool,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::Zstd,
            level: 3,
            threads: rayon::current_num_threads(),
            smart: true,
            follow_symlinks: false,
        }
    }
}

/// Application state
#[derive(Debug)]
pub struct AppState {
    /// Current mode
    pub mode: Mode,
    /// Input files/directories
    pub input_paths: Vec<PathBuf>,
    /// Output path
    pub output_path: Option<PathBuf>,
    /// Current status
    pub status: Status,
    /// Compression settings
    pub compression: CompressionSettings,
    /// Show advanced options
    pub show_advanced: bool,
    /// Log messages
    pub logs: Vec<String>,
    /// Show log window
    pub show_logs: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: Mode::Idle,
            input_paths: Vec::new(),
            output_path: None,
            status: Status::Ready,
            compression: CompressionSettings::default(),
            show_advanced: false,
            logs: Vec::new(),
            show_logs: false,
        }
    }
}

impl AppState {
    /// Clear input paths
    pub fn clear_inputs(&mut self) {
        self.input_paths.clear();
        self.output_path = None;
        self.mode = Mode::Idle;
    }
    
    /// Add input path and auto-detect mode
    pub fn add_input(&mut self, path: PathBuf) {
        self.input_paths.push(path.clone());
        
        // Auto-detect mode based on first input
        if self.mode == Mode::Idle && self.input_paths.len() == 1 {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if matches!(ext_str.as_str(), "zip" | "tar" | "gz" | "zst" | "xz" | "7z" | "br") {
                    self.mode = Mode::Extract;
                    // Set default output to current directory
                    if self.output_path.is_none() {
                        self.output_path = Some(PathBuf::from("."));
                    }
                } else {
                    self.mode = Mode::Pack;
                }
            } else {
                self.mode = Mode::Pack;
            }
        }
    }
    
    /// Check if ready to execute operation
    pub fn is_ready(&self) -> bool {
        !self.input_paths.is_empty() 
            && self.output_path.is_some() 
            && matches!(self.status, Status::Ready | Status::Success(_) | Status::Error(_))
    }
}