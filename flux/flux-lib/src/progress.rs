//! Progress reporting module

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::Duration;

/// Progress reporter for archiving operations
pub struct ProgressReporter {
    multi: Arc<MultiProgress>,
    main_bar: Option<ProgressBar>,
    file_bar: Option<ProgressBar>,
    enabled: bool,
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new(enabled: bool) -> Self {
        if enabled {
            let multi = Arc::new(MultiProgress::new());
            Self {
                multi,
                main_bar: None,
                file_bar: None,
                enabled,
            }
        } else {
            Self {
                multi: Arc::new(MultiProgress::new()),
                main_bar: None,
                file_bar: None,
                enabled: false,
            }
        }
    }

    /// Start a main progress bar for the overall operation
    pub fn start_main(&mut self, message: &str, total: u64) {
        if !self.enabled {
            return;
        }

        let bar = self.multi.add(ProgressBar::new(total));
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} {msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .unwrap()
                .progress_chars("#>-"),
        );
        bar.set_message(message.to_string());
        bar.enable_steady_tick(Duration::from_millis(100));
        self.main_bar = Some(bar);
    }

    /// Start a file progress bar
    pub fn start_file_progress(&mut self, message: &str, total: u64) {
        if !self.enabled {
            return;
        }

        let bar = self.multi.add(ProgressBar::new(total));
        bar.set_style(
            ProgressStyle::default_bar()
                .template("  {spinner:.yellow} {msg} [{bar:30.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
                .unwrap()
                .progress_chars("=>-"),
        );
        bar.set_message(message.to_string());
        bar.enable_steady_tick(Duration::from_millis(100));
        self.file_bar = Some(bar);
    }

    /// Update main progress
    pub fn inc_main(&self, delta: u64) {
        if let Some(bar) = &self.main_bar {
            bar.inc(delta);
        }
    }

    /// Update file progress
    pub fn inc_file(&self, delta: u64) {
        if let Some(bar) = &self.file_bar {
            bar.inc(delta);
        }
    }

    /// Set main message
    pub fn set_main_message(&self, message: &str) {
        if let Some(bar) = &self.main_bar {
            bar.set_message(message.to_string());
        }
    }

    /// Set file message
    pub fn set_file_message(&self, message: &str) {
        if let Some(bar) = &self.file_bar {
            bar.set_message(message.to_string());
        }
    }

    /// Finish main progress
    pub fn finish_main(&mut self) {
        if let Some(bar) = self.main_bar.take() {
            bar.finish_with_message("Complete");
        }
    }

    /// Finish file progress
    pub fn finish_file(&mut self) {
        if let Some(bar) = self.file_bar.take() {
            bar.finish_and_clear();
        }
    }

    /// Create a spinner for indeterminate progress
    pub fn spinner(&self, message: &str) -> Option<ProgressBar> {
        if !self.enabled {
            return None;
        }

        let spinner = self.multi.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(100));
        Some(spinner)
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        self.finish_main();
        self.finish_file();
    }
}

/// Simple progress callback for operations
pub trait ProgressCallback: Send + Sync {
    /// Called when progress is made
    fn progress(&self, current: u64, total: u64);

    /// Called when a new file is being processed
    fn file_progress(&self, file_name: &str, current: u64, total: u64);
}

/// No-op progress callback
pub struct NoProgressCallback;

impl ProgressCallback for NoProgressCallback {
    fn progress(&self, _current: u64, _total: u64) {}
    fn file_progress(&self, _file_name: &str, _current: u64, _total: u64) {}
}

/// Progress callback that updates a progress reporter
pub struct ReporterProgressCallback {
    reporter: Arc<std::sync::Mutex<ProgressReporter>>,
}

impl ReporterProgressCallback {
    pub fn new(reporter: ProgressReporter) -> Self {
        Self {
            reporter: Arc::new(std::sync::Mutex::new(reporter)),
        }
    }
}

impl ProgressCallback for ReporterProgressCallback {
    fn progress(&self, current: u64, _total: u64) {
        if let Ok(reporter) = self.reporter.lock() {
            if current > 0 {
                reporter.inc_main(1);
            }
        }
    }

    fn file_progress(&self, file_name: &str, current: u64, total: u64) {
        if let Ok(mut reporter) = self.reporter.lock() {
            if current == 0 {
                reporter.start_file_progress(file_name, total);
            } else if current >= total {
                reporter.finish_file();
            } else {
                reporter.inc_file(current);
            }
        }
    }
}
