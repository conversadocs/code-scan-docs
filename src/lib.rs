//! Code Scan Docs (CSD) library.
//!
//! A comprehensive library for scanning, analyzing, and documenting code repositories.
//! This library provides functionality for code analysis, plugin management, and
//! command-line interface components.
//!
//! # Modules
//!
//! - [`cli`] - Command-line interface components and argument parsing
//! - [`core`] - Core functionality for code scanning and analysis
//! - [`plugins`] - Plugin system for extensible code analysis
//! - [`utils`] - Utility functions and helper modules

pub mod cli;
pub mod core;
pub mod plugins;
pub mod utils;
pub mod context;
