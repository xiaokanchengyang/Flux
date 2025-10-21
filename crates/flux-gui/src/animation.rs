//! Animation system for smooth UI transitions

use egui::{Context, Id};
use std::collections::HashMap;
use std::time::Instant;

/// Animation controller for managing UI transitions
#[derive(Clone)]
pub struct AnimationController {
    transitions: HashMap<String, Transition>,
}

#[derive(Clone)]
pub struct Transition {
    start_time: Instant,
    duration: f32,
    from: f32,
    to: f32,
    easing: EasingFunction,
}

#[derive(Clone, Copy)]
pub enum EasingFunction {
    Linear,
    EaseInOut,
    EaseOut,
    EaseIn,
    Spring { tension: f32, friction: f32 },
    Bounce,
}

impl AnimationController {
    pub fn new() -> Self {
        Self {
            transitions: HashMap::new(),
        }
    }

    /// Start a new transition animation
    pub fn start_transition(&mut self, key: &str, from: f32, to: f32, duration: f32) {
        self.start_transition_with_easing(key, from, to, duration, EasingFunction::EaseInOut);
    }

    /// Start a transition with custom easing
    pub fn start_transition_with_easing(
        &mut self,
        key: &str,
        from: f32,
        to: f32,
        duration: f32,
        easing: EasingFunction,
    ) {
        self.transitions.insert(
            key.to_string(),
            Transition {
                start_time: Instant::now(),
                duration,
                from,
                to,
                easing,
            },
        );
    }

    /// Get the current value of an animation
    pub fn get_value(&self, key: &str) -> f32 {
        if let Some(transition) = self.transitions.get(key) {
            let elapsed = transition.start_time.elapsed().as_secs_f32();
            let progress = (elapsed / transition.duration).clamp(0.0, 1.0);

            let eased_progress = apply_easing(progress, transition.easing);
            transition.from + (transition.to - transition.from) * eased_progress
        } else {
            0.0
        }
    }

    /// Check if an animation is currently running
    pub fn is_animating(&self, key: &str) -> bool {
        if let Some(transition) = self.transitions.get(key) {
            transition.start_time.elapsed().as_secs_f32() < transition.duration
        } else {
            false
        }
    }

    /// Clean up finished animations
    pub fn cleanup(&mut self) {
        self.transitions.retain(|_, transition| {
            transition.start_time.elapsed().as_secs_f32() < transition.duration
        });
    }

    /// Check if any animation is running
    pub fn is_any_animating(&self) -> bool {
        self.transitions.values().any(|transition| {
            transition.start_time.elapsed().as_secs_f32() < transition.duration
        })
    }
}

/// Apply easing function to progress value
fn apply_easing(t: f32, easing: EasingFunction) -> f32 {
    match easing {
        EasingFunction::Linear => t,
        EasingFunction::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        EasingFunction::EaseOut => 1.0 - (1.0 - t).powi(2),
        EasingFunction::EaseIn => t * t,
        EasingFunction::Spring { tension, friction } => {
            let damped = (-t * friction).exp();
            1.0 - damped * (t * tension).cos()
        }
        EasingFunction::Bounce => {
            if t < 0.5 {
                8.0 * t * t * t * t
            } else {
                let t = t - 1.0;
                1.0 - 8.0 * t * t * t * t
            }
        }
    }
}

/// Helper trait for animating values in egui
pub trait Animate {
    /// Animate a value with the given context and key
    fn animate_value(&self, ctx: &Context, key: &str, target: f32, duration: f32) -> f32;
    
    /// Animate a boolean value (0.0 or 1.0)
    fn animate_bool(&self, ctx: &Context, key: &str, target: bool, duration: f32) -> f32;
}

impl Animate for Context {
    fn animate_value(&self, ctx: &Context, key: &str, target: f32, duration: f32) -> f32 {
        let id = Id::new(key);
        
        // Get or create animation controller
        let mut controller = ctx.data_mut(|d| {
            d.get_temp::<AnimationController>(id)
                .unwrap_or_else(AnimationController::new)
        });
        
        // Get current value
        let current = controller.get_value(key);
        
        // Start new transition if target changed
        if (current - target).abs() > 0.001 && !controller.is_animating(key) {
            controller.start_transition(key, current, target, duration);
        }
        
        // Update controller
        let value = controller.get_value(key);
        controller.cleanup();
        
        // Store controller back
        ctx.data_mut(|d| d.insert_temp(id, controller.clone()));
        
        // Request repaint if animating
        if controller.is_animating(key) {
            ctx.request_repaint();
        }
        
        value
    }
    
    fn animate_bool(&self, ctx: &Context, key: &str, target: bool, duration: f32) -> f32 {
        self.animate_value(ctx, key, if target { 1.0 } else { 0.0 }, duration)
    }
}

/// Stagger animation helper for lists
pub struct StaggerAnimation {
    base_delay: f32,
    item_delay: f32,
}

impl StaggerAnimation {
    pub fn new(base_delay: f32, item_delay: f32) -> Self {
        Self {
            base_delay,
            item_delay,
        }
    }
    
    pub fn get_delay(&self, index: usize) -> f32 {
        self.base_delay + (index as f32 * self.item_delay)
    }
}

impl Default for AnimationController {
    fn default() -> Self {
        Self::new()
    }
}