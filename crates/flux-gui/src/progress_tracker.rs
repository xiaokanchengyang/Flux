//! Progress tracking with speed and ETA calculation

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// A progress tracker that calculates speed and ETA
pub struct ProgressTracker {
    /// Start time of the operation
    start_time: Instant,
    /// Last update time
    last_update: Instant,
    /// History of speed measurements for smoothing
    speed_history: VecDeque<f64>,
    /// Maximum number of speed samples to keep
    max_samples: usize,
    /// Last processed bytes count
    last_bytes: u64,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_update: Instant::now(),
            speed_history: VecDeque::with_capacity(10),
            max_samples: 10,
            last_bytes: 0,
        }
    }

    /// Update progress and calculate speed/ETA
    pub fn update(&mut self, processed_bytes: u64, total_bytes: u64) -> (f64, Option<f64>) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);

        // Calculate instantaneous speed
        if elapsed.as_secs_f64() > 0.1 {
            // Only update if enough time has passed
            let bytes_delta = processed_bytes.saturating_sub(self.last_bytes) as f64;
            let speed = bytes_delta / elapsed.as_secs_f64();

            // Add to history
            self.speed_history.push_back(speed);
            if self.speed_history.len() > self.max_samples {
                self.speed_history.pop_front();
            }

            self.last_update = now;
            self.last_bytes = processed_bytes;
        }

        // Calculate average speed
        let avg_speed = if self.speed_history.is_empty() {
            // Fallback to overall average if no samples yet
            let total_elapsed = now.duration_since(self.start_time).as_secs_f64();
            if total_elapsed > 0.0 {
                processed_bytes as f64 / total_elapsed
            } else {
                0.0
            }
        } else {
            self.speed_history.iter().sum::<f64>() / self.speed_history.len() as f64
        };

        // Calculate ETA
        let eta = if avg_speed > 0.0 && processed_bytes < total_bytes {
            let remaining_bytes = (total_bytes - processed_bytes) as f64;
            Some(remaining_bytes / avg_speed)
        } else {
            None
        };

        (avg_speed, eta)
    }

    /// Reset the tracker
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.last_update = Instant::now();
        self.speed_history.clear();
        self.last_bytes = 0;
    }
}

/// Format bytes per second as human-readable string
pub fn format_speed(bps: f64) -> String {
    if bps < 1024.0 {
        format!("{:.0} B/s", bps)
    } else if bps < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else if bps < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bps / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB/s", bps / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Format duration as human-readable string
pub fn format_duration(seconds: f64) -> String {
    let duration = Duration::from_secs_f64(seconds);
    let total_seconds = duration.as_secs();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        if seconds > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}m", minutes)
        }
    } else {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if minutes > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    }
}
