// Top-level modules (kept for backward imports inside the crate)
// Physical grouping by core concepts
pub mod conf;
pub mod data;

pub mod cli;
//pub mod init;
pub mod engine;
pub mod rescue;
pub mod rule;
pub mod self_update;
pub mod sinks;
pub mod sources;
pub mod stat;
pub mod validate;

// Keep project doctor at top-level
pub mod project;

// KnowDB 工具
pub mod knowdb;
