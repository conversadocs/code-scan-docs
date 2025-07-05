//! Plugin system for extensible code analysis and output generation.
//!
//! This module provides a comprehensive plugin architecture that allows the application
//! to be extended with custom analyzers for different programming languages and
//! generators for various output formats.
//!
//! # Modules
//!
//! - [`communication`] - Low-level communication layer for executing plugins as external processes
//! - [`interface`] - Plugin interface definitions, data structures, and communication protocols
//! - [`manager`] - High-level plugin discovery, validation, and lifecycle management

pub mod communication;
pub mod interface;
pub mod manager;
