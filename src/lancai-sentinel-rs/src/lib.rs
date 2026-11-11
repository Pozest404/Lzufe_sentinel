#[cfg(target_os = "android")]
pub mod android;

pub mod config;
pub mod diagnostics;
pub mod enroll;
pub mod model;
pub mod net;
pub mod pipeline;
pub mod rule_engine;
pub mod schedule;
pub mod store;
pub mod text;
pub mod timetable;
