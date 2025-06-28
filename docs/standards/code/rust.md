# Rust Standards

This document outlines the specific coding standards and best practices for Rust development.

## Overview

Rust is our systems programming language of choice for performance-critical applications, CLI tools, and WebAssembly modules. These standards ensure consistent, safe, and idiomatic Rust code across all projects.

## Rust Version and Toolchain

### Minimum Supported Rust Version (MSRV)

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.70.0"  # Minimum supported version
components = ["rustfmt", "clippy", "rust-src"]
targets = ["x86_64-unknown-linux-gnu", "wasm32-unknown-unknown"]
```

### Edition and Features

```toml
# Cargo.toml
[package]
name = "my-project"
version = "0.1.0"
edition = "2021"  # Use latest stable edition
rust-version = "1.70.0"  # MSRV declaration

[features]
default = ["tokio-runtime"]
tokio-runtime = ["tokio"]
blocking = []
```

## Code Formatting

### Rustfmt Configuration

```toml
# rustfmt.toml
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
merge_derives = true
use_try_shorthand = true
use_field_init_shorthand = true
force_explicit_abi = true
normalize_comments = true
wrap_comments = true
format_code_in_doc_comments = true
comment_width = 80
normalize_doc_attributes = true
```

### Code Style Rules

```rust
// Use 4 spaces for indentation
fn calculate_total(items: &[Item]) -> u32 {
    items
        .iter()
        .filter(|item| item.is_active)
        .map(|item| item.price)
        .sum()
}

// Line length: 100 characters max
fn very_long_function_name_that_demonstrates_line_wrapping(
    parameter_one: String,
    parameter_two: i32,
    parameter_three: Option<bool>,
) -> Result<String, Error> {
    // implementation
    Ok(format!("{}-{}-{:?}", parameter_one, parameter_two, parameter_three))
}

// Use trailing commas in multi-line structures
let config = Config {
    database_url: "postgresql://localhost/myapp".to_string(),
    redis_url: "redis://localhost".to_string(),
    log_level: LogLevel::Info,
    max_connections: 100, // <- trailing comma
};
```

## Naming Conventions

### Variables and Functions

```rust
// snake_case for variables and functions
let user_name = "john_doe";
let total_amount = 100.50;

fn get_user_by_id(user_id: u64) -> Option<User> {
    // implementation
    None
}

fn calculate_monthly_revenue() -> Result<f64, CalculationError> {
    // implementation
    Ok(0.0)
}

// Boolean variables should be descriptive
let is_authenticated = true;
let has_permission = false;
let can_edit = user.role == Role::Admin;
```

### Types and Constants

```rust
// PascalCase for types (structs, enums, traits)
struct UserProfile {
    id: u64,
    email: String,
    name: String,
    created_at: DateTime<Utc>,
}

enum UserRole {
    Admin,
    Moderator,
    User,
}

trait UserRepository {
    fn find_by_id(&self, id: u64) -> Result<Option<User>, Error>;
}

// SCREAMING_SNAKE_CASE for constants
const MAX_RETRY_ATTEMPTS: u32 = 3;
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const API_BASE_URL: &str = "https://api.example.com";

// Module names use snake_case
mod user_service;
mod database_utils;
```

### Lifetimes and Generics

```rust
// Single lowercase letters for lifetimes
struct UserRef<'a> {
    name: &'a str,
    email: &'a str,
}

// Single uppercase letters for generic types
struct Repository<T, E> {
    connection: T,
    _error: PhantomData<E>,
}

// Descriptive names for complex generics
trait AsyncRepository<Entity, Error> {
    async fn save(&mut self, entity: Entity) -> Result<Entity, Error>;
}
```

## Type System Best Practices

### Error Handling

```rust
use thiserror::Error;

// Define specific error types
#[derive(Error, Debug)]
pub enum UserError {
    #[error("User with ID {id} not found")]
    NotFound { id: u64 },

    #[error("Invalid email address: {email}")]
    InvalidEmail { email: String },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

// Use Result types for fallible operations
pub fn create_user(user_data: CreateUserRequest) -> Result<User, UserError> {
    // Validate email
    if !is_valid_email(&user_data.email) {
        return Err(UserError::InvalidEmail {
            email: user_data.email,
        });
    }

    // Create user
    let user = User {
        id: generate_id(),
        email: user_data.email,
        name: user_data.name,
        created_at: Utc::now(),
    };

    Ok(user)
}

// Use ? operator for error propagation
pub async fn get_user_profile(id: u64) -> Result<UserProfile, UserError> {
    let user = get_user_by_id(id).await?;
    let profile = fetch_user_profile(user.id).await?;
    Ok(profile)
}
```

### Option Handling

```rust
// Use combinators instead of explicit matching when possible
fn get_user_display_name(user: &Option<User>) -> String {
    user.as_ref()
        .map(|u| &u.name)
        .unwrap_or("Anonymous")
        .to_string()
}

// Pattern matching for complex Option handling
fn process_user_data(user: Option<User>) -> ProcessResult {
    match user {
        Some(user) if user.is_active => ProcessResult::Success(user.id),
        Some(user) => ProcessResult::Inactive(user.id),
        None => ProcessResult::NotFound,
    }
}

// Use ok_or for converting Option to Result
fn find_user_or_error(id: u64) -> Result<User, UserError> {
    find_user(id).ok_or(UserError::NotFound { id })
}
```

### Ownership and Borrowing

```rust
// Prefer borrowing over taking ownership when possible
fn calculate_total_price(items: &[Item]) -> f64 {
    items.iter().map(|item| item.price).sum()
}

// Use Cow for conditional ownership
use std::borrow::Cow;

fn normalize_email(email: &str) -> Cow<str> {
    if email.chars().any(|c| c.is_uppercase()) {
        Cow::Owned(email.to_lowercase())
    } else {
        Cow::Borrowed(email)
    }
}

// Use smart pointers appropriately
use std::sync::Arc;
use std::rc::Rc;

// Arc for shared ownership across threads
type SharedConfig = Arc<Config>;

// Rc for shared ownership within single thread
type LocalCache = Rc<RefCell<HashMap<String, Value>>>;
```

## Struct and Enum Design

### Struct Patterns

```rust
// Use builder pattern for complex initialization
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub timeout: Duration,
    pub ssl_mode: SslMode,
}

impl DatabaseConfig {
    pub fn builder() -> DatabaseConfigBuilder {
        DatabaseConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct DatabaseConfigBuilder {
    url: Option<String>,
    max_connections: Option<u32>,
    timeout: Option<Duration>,
    ssl_mode: Option<SslMode>,
}

impl DatabaseConfigBuilder {
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = Some(max);
        self
    }

    pub fn build(self) -> Result<DatabaseConfig, ConfigError> {
        Ok(DatabaseConfig {
            url: self.url.ok_or(ConfigError::MissingUrl)?,
            max_connections: self.max_connections.unwrap_or(10),
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
            ssl_mode: self.ssl_mode.unwrap_or(SslMode::Prefer),
        })
    }
}
```

### Enum Patterns

```rust
// Use enums for state machines
#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Pending { created_at: DateTime<Utc> },
    Processing { started_at: DateTime<Utc> },
    Shipped {
        shipped_at: DateTime<Utc>,
        tracking_number: String,
    },
    Delivered {
        delivered_at: DateTime<Utc>,
        signature: Option<String>,
    },
    Cancelled {
        cancelled_at: DateTime<Utc>,
        reason: String,
    },
}

impl OrderStatus {
    pub fn can_transition_to(&self, new_status: &OrderStatus) -> bool {
        use OrderStatus::*;
        matches!(
            (self, new_status),
            (Pending { .. }, Processing { .. })
                | (Processing { .. }, Shipped { .. })
                | (Shipped { .. }, Delivered { .. })
                | (Pending { .. } | Processing { .. }, Cancelled { .. })
        )
    }
}

// Use enums for error handling with context
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Invalid request: {message}")]
    BadRequest { message: String },

    #[error("Resource not found: {resource_type} with ID {id}")]
    NotFound { resource_type: String, id: String },

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}
```

## Async Programming

### Async Function Patterns

```rust
use tokio::time::{sleep, Duration, timeout};
use futures::{stream, StreamExt, TryStreamExt};

// Use async/await for I/O operations
pub async fn fetch_user_data(user_id: u64) -> Result<UserData, ApiError> {
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("https://api.example.com/users/{}", user_id))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(ApiError::HttpError(response.status()));
    }

    let user_data: UserData = response.json().await?;
    Ok(user_data)
}

// Concurrent processing with proper error handling
pub async fn fetch_multiple_users(user_ids: Vec<u64>) -> Result<Vec<UserData>, ApiError> {
    let tasks = user_ids
        .into_iter()
        .map(|id| tokio::spawn(fetch_user_data(id)));

    let mut results = Vec::new();
    for task in tasks {
        match task.await? {
            Ok(user_data) => results.push(user_data),
            Err(e) => log::warn!("Failed to fetch user data: {}", e),
        }
    }

    Ok(results)
}

// Stream processing for large datasets
pub async fn process_user_stream(
    users: impl Stream<Item = Result<User, DatabaseError>> + Unpin,
) -> Result<ProcessingSummary, ProcessingError> {
    let mut summary = ProcessingSummary::default();

    let mut stream = users.chunks(100); // Process in batches

    while let Some(batch) = stream.next().await {
        let valid_users: Vec<User> = batch
            .into_iter()
            .filter_map(|result| {
                match result {
                    Ok(user) => Some(user),
                    Err(e) => {
                        summary.errors += 1;
                        log::error!("Database error: {}", e);
                        None
                    }
                }
            })
            .collect();

        if !valid_users.is_empty() {
            process_user_batch(&valid_users).await?;
            summary.processed += valid_users.len();
        }
    }

    Ok(summary)
}
```

### Channel and Concurrency Patterns

```rust
use tokio::sync::{mpsc, oneshot, RwLock};
use std::sync::Arc;

// Actor pattern with message passing
pub struct UserServiceActor {
    receiver: mpsc::Receiver<UserMessage>,
    repository: Arc<dyn UserRepository + Send + Sync>,
}

pub enum UserMessage {
    GetUser {
        id: u64,
        respond_to: oneshot::Sender<Result<User, UserError>>,
    },
    CreateUser {
        data: CreateUserRequest,
        respond_to: oneshot::Sender<Result<User, UserError>>,
    },
}

impl UserServiceActor {
    pub fn new(
        repository: Arc<dyn UserRepository + Send + Sync>,
    ) -> (Self, mpsc::Sender<UserMessage>) {
        let (sender, receiver) = mpsc::channel(100);
        let actor = Self { receiver, repository };
        (actor, sender)
    }

    pub async fn run(mut self) {
        while let Some(message) = self.receiver.recv().await {
            match message {
                UserMessage::GetUser { id, respond_to } => {
                    let result = self.repository.find_by_id(id).await;
                    let _ = respond_to.send(result);
                }
                UserMessage::CreateUser { data, respond_to } => {
                    let result = self.repository.create(data).await;
                    let _ = respond_to.send(result);
                }
            }
        }
    }
}

// Shared state with proper synchronization
#[derive(Clone)]
pub struct CacheService {
    cache: Arc<RwLock<HashMap<String, CachedValue>>>,
}

impl CacheService {
    pub async fn get(&self, key: &str) -> Option<CachedValue> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    pub async fn set(&self, key: String, value: CachedValue) {
        let mut cache = self.cache.write().await;
        cache.insert(key, value);
    }
}
```

## Testing Standards

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[test]
    fn test_user_creation_with_valid_data() {
        let user_data = CreateUserRequest {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        let result = create_user(user_data);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.name, "Test User");
    }

    #[test]
    fn test_user_creation_with_invalid_email() {
        let user_data = CreateUserRequest {
            email: "invalid-email".to_string(),
            name: "Test User".to_string(),
        };

        let result = create_user(user_data);

        assert!(result.is_err());
        match result.unwrap_err() {
            UserError::InvalidEmail { email } => {
                assert_eq!(email, "invalid-email");
            }
            _ => panic!("Expected InvalidEmail error"),
        }
    }

    #[tokio::test]
    async fn test_async_user_fetch() {
        let user_id = 123;

        // Use a mock or test database
        let result = fetch_user_data(user_id).await;

        assert!(result.is_ok());
    }

    // Property-based testing with proptest
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_email_normalization(email in "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}") {
            let normalized = normalize_email(&email);
            assert!(is_valid_email(&normalized));
        }
    }
}
```

### Integration Testing

```rust
// tests/integration_test.rs
use my_app::{Database, UserService, Config};
use sqlx::PgPool;
use testcontainers::*;

#[tokio::test]
async fn test_user_service_integration() {
    // Set up test database
    let docker = clients::Cli::default();
    let postgres_image = images::postgres::Postgres::default();
    let node = docker.run(postgres_image);

    let connection_string = format!(
        "postgresql://postgres:postgres@127.0.0.1:{}/postgres",
        node.get_host_port_ipv4(5432)
    );

    let pool = PgPool::connect(&connection_string).await.unwrap();

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Test the service
    let database = Database::new(pool);
    let user_service = UserService::new(database);

    let user_data = CreateUserRequest {
        email: "integration@test.com".to_string(),
        name: "Integration Test".to_string(),
    };

    let created_user = user_service.create_user(user_data).await.unwrap();
    let fetched_user = user_service.get_user(created_user.id).await.unwrap();

    assert_eq!(created_user.id, fetched_user.id);
    assert_eq!(created_user.email, fetched_user.email);
}
```

### Benchmark Testing

```rust
// benches/user_operations.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use my_app::*;

fn benchmark_user_creation(c: &mut Criterion) {
    c.bench_function("create_user", |b| {
        b.iter(|| {
            let user_data = CreateUserRequest {
                email: black_box("bench@example.com".to_string()),
                name: black_box("Benchmark User".to_string()),
            };
            create_user(user_data)
        })
    });
}

fn benchmark_email_validation(c: &mut Criterion) {
    let emails = vec![
        "valid@example.com",
        "another.valid+email@subdomain.example.org",
        "invalid.email",
        "missing@domain",
    ];

    c.bench_function("validate_emails", |b| {
        b.iter(|| {
            for email in &emails {
                black_box(is_valid_email(black_box(email)));
            }
        })
    });
}

criterion_group!(benches, benchmark_user_creation, benchmark_email_validation);
criterion_main!(benches);
```

## Documentation Standards

### Code Documentation

````rust
//! # User Management Module
//!
//! This module provides functionality for managing user accounts,
//! including creation, authentication, and profile management.
//!
//! ## Examples
//!
//! ```rust
//! use my_app::user::{UserService, CreateUserRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let user_service = UserService::new(database);
//!
//! let request = CreateUserRequest {
//!     email: "user@example.com".to_string(),
//!     name: "John Doe".to_string(),
//! };
//!
//! let user = user_service.create_user(request).await?;
//! println!("Created user with ID: {}", user.id);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

/// Represents a user in the system.
///
/// A user has a unique identifier, email address, display name,
/// and creation timestamp. All users are created in an active state.
///
/// # Examples
///
/// ```rust
/// use chrono::Utc;
/// use my_app::User;
///
/// let user = User {
///     id: 1,
///     email: "user@example.com".to_string(),
///     name: "John Doe".to_string(),
///     created_at: Utc::now(),
/// };
///
/// assert_eq!(user.email, "user@example.com");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier for the user
    pub id: u64,

    /// User's email address (must be unique across the system)
    pub email: String,

    /// User's display name
    pub name: String,

    /// Timestamp when the user was created
    pub created_at: DateTime<Utc>,
}

/// Service for managing user operations.
///
/// This service provides high-level operations for user management,
/// including creation, retrieval, and validation. It handles business
/// logic and coordinates with the underlying repository.
///
/// # Examples
///
/// ```rust
/// # use my_app::{UserService, Database};
/// # async fn example() {
/// let database = Database::connect("postgresql://...").await.unwrap();
/// let user_service = UserService::new(database);
///
/// // Create a new user
/// let request = CreateUserRequest {
///     email: "new@example.com".to_string(),
///     name: "New User".to_string(),
/// };
/// let user = user_service.create_user(request).await.unwrap();
/// # }
/// ```
pub struct UserService {
    repository: Box<dyn UserRepository + Send + Sync>,
}

impl UserService {
    /// Creates a new user service with the given repository.
    ///
    /// # Arguments
    ///
    /// * `repository` - The repository implementation for user data persistence
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use my_app::{UserService, InMemoryUserRepository};
    /// let repository = InMemoryUserRepository::new();
    /// let service = UserService::new(Box::new(repository));
    /// ```
    pub fn new(repository: Box<dyn UserRepository + Send + Sync>) -> Self {
        Self { repository }
    }

    /// Creates a new user with the provided data.
    ///
    /// This method validates the input data, ensures the email is unique,
    /// and creates a new user record. The user is assigned a unique ID
    /// and creation timestamp.
    ///
    /// # Arguments
    ///
    /// * `request` - The user creation request containing email and name
    ///
    /// # Returns
    ///
    /// * `Ok(User)` - The created user with assigned ID and timestamp
    /// * `Err(UserError)` - If validation fails or the email already exists
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// * The email address is invalid
    /// * A user with the same email already exists
    /// * The database operation fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use my_app::{UserService, CreateUserRequest, InMemoryUserRepository};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let repository = InMemoryUserRepository::new();
    /// let service = UserService::new(Box::new(repository));
    ///
    /// let request = CreateUserRequest {
    ///     email: "user@example.com".to_string(),
    ///     name: "John Doe".to_string(),
    /// };
    ///
    /// let user = service.create_user(request).await?;
    /// assert_eq!(user.email, "user@example.com");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, UserError> {
        // Validate email format
        if !is_valid_email(&request.email) {
            return Err(UserError::InvalidEmail {
                email: request.email,
            });
        }

        // Check if user already exists
        if let Some(_existing) = self.repository.find_by_email(&request.email).await? {
            return Err(UserError::EmailAlreadyExists {
                email: request.email,
            });
        }

        // Create the user
        let user = User {
            id: self.generate_user_id().await?,
            email: request.email,
            name: request.name,
            created_at: Utc::now(),
        };

        self.repository.save(&user).await?;
        Ok(user)
    }
}
````

### README Documentation

````markdown
# Project Name

[![Crates.io](https://img.shields.io/crates/v/my-project.svg)](https://crates.io/crates/my-project)
[![Documentation](https://docs.rs/my-project/badge.svg)](https://docs.rs/my-project)
[![Build Status](https://github.com/username/my-project/workflows/CI/badge.svg)](https://github.com/username/my-project/actions)

A brief description of what this project does and who it's for.

## Features

- 🚀 Fast and safe systems programming
- 🔒 Memory safety without garbage collection
- ⚡ Zero-cost abstractions
- 🌐 WebAssembly support

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
my-project = "0.1.0"
```
````

## Usage

```rust
use my_project::UserService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = UserService::new(database).await?;

    let user = service.create_user(CreateUserRequest {
        email: "user@example.com".to_string(),
        name: "John Doe".to_string(),
    }).await?;

    println!("Created user: {}", user.name);
    Ok(())
}
```

## Performance

This library is designed for high performance:

- Zero-copy string operations where possible
- Async/await for non-blocking I/O
- Efficient memory usage with smart pointers

## Safety

All public APIs are safe Rust. Unsafe code is used internally only where necessary for performance and is thoroughly tested.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

````

## Dependency Management

### Cargo.toml Structure
```toml
[package]
name = "my-project"
version = "0.1.0"
edition = "2021"
rust-version = "1.70.0"
authors = ["Your Name <your.email@example.com>"]
license = "MIT OR Apache-2.0"
description = "A brief description of the project"
homepage = "https://github.com/username/my-project"
repository = "https://github.com/username/my-project"
documentation = "https://docs.rs/my-project"
keywords = ["async", "web", "api"]
categories = ["web-programming", "asynchronous"]
readme = "README.md"
exclude = ["tests/fixtures/*", "benches/data/*"]

[dependencies]
# Async runtime
tokio = { version = "1.0", features = ["full"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid"] }

# HTTP client/server
reqwest = { version = "0.11", features = ["json"] }
axum = "0.7"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tokio-test = "0.4"
proptest = "1.0"
criterion = { version = "0.5", features = ["html_reports"] }
testcontainers = "0.15"

[features]
default = ["tokio-runtime"]
tokio-runtime = ["tokio"]
blocking = []

# Optional features for different use cases
metrics = ["prometheus"]
tracing = ["tracing-subscriber/fmt"]

[[bin]]
name = "my-project-cli"
path = "src/bin/cli.rs"

[[bench]]
name = "user_operations"
harness = false

[profile.release]
lto = true
codegen-units = 1
panic = "abort"

[profile.dev]
debug = true
opt-level = 0

[profile.test]
debug = true
opt-level = 0
````

### Version Pinning Strategy

```toml
# Pin exact versions for reproducible builds
serde = "=1.0.136"  # Exact version
tokio = "~1.21.0"   # Patch updates only
reqwest = "^0.11"   # Minor updates allowed

# Use workspace dependencies for multi-crate projects
[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

## Security Best Practices

### Input Validation

```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Validate, Deserialize)]
pub struct CreateUserRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: String,

    #[validate(range(min = 18, max = 120, message = "Age must be between 18 and 120"))]
    pub age: Option<u32>,
}

impl CreateUserRequest {
    pub fn validate_and_sanitize(mut self) -> Result<Self, ValidationError> {
        // Sanitize input
        self.email = self.email.trim().to_lowercase();
        self.name = self.name.trim().to_string();

        // Validate
        self.validate()?;

        Ok(self)
    }
}

// Custom validation functions
fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("Password must be at least 8 characters"));
    }

    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !(has_upper && has_lower && has_digit && has_special) {
        return Err(ValidationError::new(
            "Password must contain uppercase, lowercase, digit, and special character"
        ));
    }

    Ok(())
}
```

### Secret Management

```rust
use secrecy::{Secret, ExposeSecret};
use zeroize::Zeroize;

// Use Secret wrapper for sensitive data
#[derive(Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub password: Secret<String>,
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            url: env::var("DATABASE_URL")?,
            password: Secret::new(env::var("DATABASE_PASSWORD")?),
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
        })
    }

    pub fn connection_string(&self) -> String {
        format!(
            "postgresql://user:{}@host/db?max_connections={}",
            self.password.expose_secret(),
            self.max_connections
        )
    }
}

// Zeroize sensitive data structures
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct AuthToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

// Constant-time comparison for sensitive data
use subtle::ConstantTimeEq;

pub fn verify_token(provided: &str, expected: &str) -> bool {
    provided.as_bytes().ct_eq(expected.as_bytes()).into()
}
```

### Memory Safety

```rust
// Use safe abstractions over raw pointers
use std::ptr::NonNull;
use std::marker::PhantomData;

pub struct SafeBuffer<T> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
    _marker: PhantomData<T>,
}

impl<T> SafeBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let layout = std::alloc::Layout::array::<T>(capacity).unwrap();
        let ptr = unsafe {
            let ptr = std::alloc::alloc(layout) as *mut T;
            NonNull::new(ptr).expect("Failed to allocate memory")
        };

        Self {
            ptr,
            len: 0,
            capacity,
            _marker: PhantomData,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), BufferError> {
        if self.len >= self.capacity {
            return Err(BufferError::BufferFull);
        }

        unsafe {
            self.ptr.as_ptr().add(self.len).write(value);
        }
        self.len += 1;
        Ok(())
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            unsafe { Some(&*self.ptr.as_ptr().add(index)) }
        } else {
            None
        }
    }
}

impl<T> Drop for SafeBuffer<T> {
    fn drop(&mut self) {
        // Properly drop all elements
        for i in 0..self.len {
            unsafe {
                self.ptr.as_ptr().add(i).drop_in_place();
            }
        }

        // Deallocate memory
        let layout = std::alloc::Layout::array::<T>(self.capacity).unwrap();
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}
```

## Performance Optimization

### Memory Management

```rust
// Use object pools for frequent allocations
use std::sync::Mutex;

pub struct ObjectPool<T> {
    objects: Mutex<Vec<T>>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
}

impl<T> ObjectPool<T> {
    pub fn new<F>(factory: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static
    {
        Self {
            objects: Mutex::new(Vec::new()),
            factory: Box::new(factory),
        }
    }

    pub fn get(&self) -> PooledObject<T> {
        let mut objects = self.objects.lock().unwrap();
        let object = objects.pop().unwrap_or_else(|| (self.factory)());
        PooledObject::new(object, self)
    }
}

pub struct PooledObject<T> {
    object: Option<T>,
    pool: *const ObjectPool<T>,
}

impl<T> PooledObject<T> {
    fn new(object: T, pool: &ObjectPool<T>) -> Self {
        Self {
            object: Some(object),
            pool: pool as *const _,
        }
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.object.as_ref().unwrap()
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(object) = self.object.take() {
            unsafe {
                let pool = &*self.pool;
                let mut objects = pool.objects.lock().unwrap();
                objects.push(object);
            }
        }
    }
}
```

### SIMD Operations

```rust
// Use SIMD for performance-critical operations
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn sum_floats_simd(data: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { sum_floats_avx2(data) };
        }
    }

    // Fallback to regular implementation
    data.iter().sum()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn sum_floats_avx2(data: &[f32]) -> f32 {
    let mut sum = _mm256_setzero_ps();

    let chunks = data.chunks_exact(8);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let values = _mm256_loadu_ps(chunk.as_ptr());
        sum = _mm256_add_ps(sum, values);
    }

    // Extract sum from SIMD register
    let sum_array: [f32; 8] = std::mem::transmute(sum);
    let mut total = sum_array.iter().sum::<f32>();

    // Add remainder
    total += remainder.iter().sum::<f32>();

    total
}
```

### Zero-Copy Operations

```rust
use std::borrow::Cow;
use bytes::{Bytes, BytesMut, Buf, BufMut};

// Use Cow for conditional cloning
pub fn process_text(input: &str, normalize: bool) -> Cow<str> {
    if normalize {
        Cow::Owned(input.to_lowercase())
    } else {
        Cow::Borrowed(input)
    }
}

// Use Bytes for zero-copy buffer management
pub struct MessageBuffer {
    data: Bytes,
}

impl MessageBuffer {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Bytes::from(data),
        }
    }

    pub fn slice(&self, start: usize, end: usize) -> Bytes {
        self.data.slice(start..end)  // Zero-copy slice
    }

    pub fn split_to(&mut self, at: usize) -> Bytes {
        self.data.split_to(at)  // Zero-copy split
    }
}

// Efficient string building
pub fn build_query(table: &str, conditions: &[(&str, &str)]) -> String {
    let mut query = String::with_capacity(
        64 + table.len() + conditions.len() * 32
    );

    query.push_str("SELECT * FROM ");
    query.push_str(table);

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        for (i, (key, value)) in conditions.iter().enumerate() {
            if i > 0 {
                query.push_str(" AND ");
            }
            query.push_str(key);
            query.push_str(" = '");
            query.push_str(value);
            query.push('\'');
        }
    }

    query
}
```

## Configuration Files

### Clippy Configuration

```toml
# clippy.toml
# Clippy configuration for stricter linting

# Complexity lints
cognitive-complexity-threshold = 30
too-many-lines-threshold = 100

# Style preferences
single-char-add-str = true
trivial-copy-pass-by-ref = true

# Performance lints
large-error-threshold = 128
trivial-copy-pass-by-ref = true

# Pedantic lints to enable
doc-markdown = true
missing-errors-doc = true
missing-panics-doc = true
```

### Cargo Configuration

```toml
# .cargo/config.toml
[build]
rustflags = [
    "-D", "warnings",           # Treat warnings as errors
    "-D", "rust-2018-idioms",   # Enforce 2018 idioms
    "-D", "clippy::all",        # All clippy lints
    "-D", "clippy::pedantic",   # Pedantic clippy lints
    "-A", "clippy::module-name-repetitions",  # Allow some repetition
]

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

# For faster compilation during development
[profile.dev]
debug = 1  # Reduced debug info for faster builds

[profile.release]
lto = "thin"  # Thin LTO for better performance
```

### GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test:
    name: Test Suite
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [stable, beta, nightly]
    steps:
      - name: Checkout sources
        uses: actions/checkout@v4

      - name: Install toolchain
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy

      - name: Run cargo check
        run: cargo check --all-targets --all-features

      - name: Run cargo test
        run: cargo test --all-targets --all-features

      - name: Run cargo clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Run cargo fmt
        run: cargo fmt --all -- --check

  coverage:
    name: Coverage
    runs-on: ubuntu-latest
    steps:
      - name: Checkout sources
        uses: actions/checkout@v4

      - name: Install toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview

      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

      - name: Upload to codecov.io
        uses: codecov/codecov-action@v3
        with:
          file: lcov.info

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - name: Checkout sources
        uses: actions/checkout@v4

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Run security audit
        run: cargo audit

  msrv:
    name: Minimum Supported Rust Version
    runs-on: ubuntu-latest
    steps:
      - name: Checkout sources
        uses: actions/checkout@v4

      - name: Install toolchain
        uses: dtolnay/rust-toolchain@1.70.0

      - name: Check MSRV
        run: cargo check --all-targets --all-features
```

## WebAssembly Integration

### WASM Module Structure

```rust
// src/wasm.rs
use wasm_bindgen::prelude::*;
use web_sys::console;

// Import JavaScript functions
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Macro for console.log
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

// Export Rust functions to JavaScript
#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

#[wasm_bindgen]
pub struct Calculator {
    value: f64,
}

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Calculator {
        console_log!("Calculator created");
        Calculator { value: 0.0 }
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn add(&mut self, x: f64) {
        self.value += x;
    }

    pub fn multiply(&mut self, x: f64) {
        self.value *= x;
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
    }
}

// Working with complex types
#[wasm_bindgen]
pub struct UserData {
    name: String,
    age: u32,
}

#[wasm_bindgen]
impl UserData {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, age: u32) -> UserData {
        UserData { name, age }
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn age(&self) -> u32 {
        self.age
    }

    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(self)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

### WASM-Specific Cargo.toml

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
web-sys = "0.3"
js-sys = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
console_error_panic_hook = "0.1"
wee_alloc = "0.4"

[dependencies.web-sys]
version = "0.3"
features = [
  "console",
  "Document",
  "Element",
  "HtmlElement",
  "Window",
]

# Use wee_alloc as the global allocator for smaller binary size
[target.'cfg(target_arch = "wasm32")'.dependencies]
wee_alloc = "0.4"

# Enable debug info in release mode for better stack traces
[profile.release]
debug = true
```

## CLI Application Patterns

### Argument Parsing with Clap

```rust
// src/cli.rs
use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "myapp")]
#[command(about = "A CLI application for managing users")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// Global verbosity level
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// User management commands
    User(UserArgs),
    /// Database operations
    Database(DatabaseArgs),
    /// Configuration management
    Config(ConfigArgs),
}

#[derive(Args)]
pub struct UserArgs {
    #[command(subcommand)]
    pub action: UserAction,
}

#[derive(Subcommand)]
pub enum UserAction {
    /// Create a new user
    Create {
        /// User's email address
        #[arg(short, long)]
        email: String,

        /// User's display name
        #[arg(short, long)]
        name: String,

        /// Skip email validation
        #[arg(long)]
        skip_validation: bool,
    },
    /// List all users
    List {
        /// Maximum number of users to display
        #[arg(short, long, default_value_t = 10)]
        limit: usize,

        /// Filter by active status
        #[arg(long)]
        active_only: bool,
    },
    /// Get user by ID
    Get {
        /// User ID
        id: u64,
    },
    /// Delete a user
    Delete {
        /// User ID
        id: u64,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Text,
    Json,
    Yaml,
}

// Main CLI handler
pub async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();

    // Load configuration
    let config = if let Some(config_path) = cli.config {
        Config::from_file(config_path)?
    } else {
        Config::default()
    };

    // Initialize services
    let database = Database::connect(&config.database_url).await?;
    let user_service = UserService::new(database);

    // Execute command
    match cli.command {
        Commands::User(user_args) => {
            handle_user_command(user_args, &user_service, cli.format).await?;
        }
        Commands::Database(db_args) => {
            handle_database_command(db_args, &config).await?;
        }
        Commands::Config(config_args) => {
            handle_config_command(config_args, &config).await?;
        }
    }

    Ok(())
}

async fn handle_user_command(
    args: UserArgs,
    service: &UserService,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match args.action {
        UserAction::Create { email, name, skip_validation } => {
            let request = CreateUserRequest {
                email,
                name,
                skip_validation,
            };

            let user = service.create_user(request).await?;
            output_user(&user, format);
        }
        UserAction::List { limit, active_only } => {
            let users = service.list_users(limit, active_only).await?;
            output_users(&users, format);
        }
        UserAction::Get { id } => {
            let user = service.get_user(id).await?;
            output_user(&user, format);
        }
        UserAction::Delete { id, force } => {
            if !force {
                print!("Are you sure you want to delete user {}? (y/N): ", id);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Deletion cancelled");
                    return Ok(());
                }
            }

            service.delete_user(id).await?;
            println!("User {} deleted successfully", id);
        }
    }

    Ok(())
}
```

### Progress Bars and Interactive CLI

```rust
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use dialoguer::{Confirm, Input, Select};
use console::Term;

pub async fn interactive_user_creation() -> Result<(), Box<dyn std::error::Error>> {
    let term = Term::stdout();
    term.clear_screen()?;

    println!("🚀 Interactive User Creation");
    println!();

    // Get user input
    let email: String = Input::new()
        .with_prompt("Email address")
        .validate_with(|input: &String| -> Result<(), &str> {
            if is_valid_email(input) {
                Ok(())
            } else {
                Err("Please enter a valid email address")
            }
        })
        .interact_text()?;

    let name: String = Input::new()
        .with_prompt("Full name")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                Err("Name cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    let role_options = vec!["User", "Moderator", "Admin"];
    let role_index = Select::new()
        .with_prompt("Select user role")
        .items(&role_options)
        .default(0)
        .interact()?;

    let send_welcome = Confirm::new()
        .with_prompt("Send welcome email?")
        .default(true)
        .interact()?;

    // Show progress during user creation
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );

    pb.set_message("Creating user...");
    pb.enable_steady_tick(Duration::from_millis(100));

    // Simulate async work
    tokio::time::sleep(Duration::from_millis(500)).await;

    pb.set_message("Validating email...");
    tokio::time::sleep(Duration::from_millis(300)).await;

    pb.set_message("Saving to database...");
    tokio::time::sleep(Duration::from_millis(400)).await;

    if send_welcome {
        pb.set_message("Sending welcome email...");
        tokio::time::sleep(Duration::from_millis(600)).await;
    }

    pb.finish_with_message("✅ User created successfully!");

    println!();
    println!("User Details:");
    println!("  Email: {}", email);
    println!("  Name: {}", name);
    println!("  Role: {}", role_options[role_index]);

    Ok(())
}
```

## Project Templates

### Makefile for Rust Projects

```makefile
# Rust Project Makefile

.PHONY: help build test clippy fmt check clean install doc bench
.DEFAULT_GOAL := help

# Colors
BLUE := \033[34m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
RESET := \033[0m

help: ## Show this help message
	@echo "$(BLUE)Rust Project Commands$(RESET)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"; printf "$(BLUE)Usage:$(RESET)\n  make $(YELLOW)<target>$(RESET)\n\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  $(YELLOW)%-15s$(RESET) %s\n", $1, $2 }' $(MAKEFILE_LIST)

build: ## Build the project
	@echo "$(BLUE)Building project...$(RESET)"
	cargo build

build-release: ## Build the project in release mode
	@echo "$(BLUE)Building project (release)...$(RESET)"
	cargo build --release

test: ## Run tests
	@echo "$(BLUE)Running tests...$(RESET)"
	cargo test

test-coverage: ## Run tests with coverage
	@echo "$(BLUE)Running tests with coverage...$(RESET)"
	cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

clippy: ## Run clippy lints
	@echo "$(BLUE)Running clippy...$(RESET)"
	cargo clippy --all-targets --all-features -- -D warnings

fmt: ## Format code
	@echo "$(BLUE)Formatting code...$(RESET)"
	cargo fmt --all

fmt-check: ## Check code formatting
	@echo "$(BLUE)Checking code formatting...$(RESET)"
	cargo fmt --all -- --check

check: ## Run all checks (clippy, fmt, test)
	@$(MAKE) clippy
	@$(MAKE) fmt-check
	@$(MAKE) test

clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(RESET)"
	cargo clean

install: ## Install the binary
	@echo "$(BLUE)Installing binary...$(RESET)"
	cargo install --path .

doc: ## Generate documentation
	@echo "$(BLUE)Generating documentation...$(RESET)"
	cargo doc --no-deps --open

bench: ## Run benchmarks
	@echo "$(BLUE)Running benchmarks...$(RESET)"
	cargo bench

audit: ## Run security audit
	@echo "$(BLUE)Running security audit...$(RESET)"
	cargo audit

update: ## Update dependencies
	@echo "$(BLUE)Updating dependencies...$(RESET)"
	cargo update

wasm-build: ## Build WebAssembly module
	@echo "$(BLUE)Building WebAssembly module...$(RESET)"
	wasm-pack build --target web --out-dir pkg

wasm-test: ## Test WebAssembly module
	@echo "$(BLUE)Testing WebAssembly module...$(RESET)"
	wasm-pack test --headless --firefox

docker-build: ## Build Docker image
	@echo "$(BLUE)Building Docker image...$(RESET)"
	docker build -t my-rust-app .

docker-run: ## Run Docker container
	@echo "$(BLUE)Running Docker container...$(RESET)"
	docker run -it --rm my-rust-app

dev: ## Start development mode (watch for changes)
	@echo "$(BLUE)Starting development mode...$(RESET)"
	cargo watch -x check -x test -x run

pre-commit: ## Run pre-commit checks
	@$(MAKE) fmt
	@$(MAKE) clippy
	@$(MAKE) test
	@echo "$(GREEN)All pre-commit checks passed!$(RESET)"
```

### Justfile Alternative

```just
# Rust project commands using just

# Default recipe
default:
    @just --list

# Build the project
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests with coverage
test-coverage:
    cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# Run clippy
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run all quality checks
check: clippy fmt-check test

# Clean build artifacts
clean:
    cargo clean

# Generate and open documentation
doc:
    cargo doc --no-deps --open

# Run benchmarks
bench:
    cargo bench

# Security audit
audit:
    cargo audit

# Update dependencies
update:
    cargo update

# Development mode with file watching
dev:
    cargo watch -x check -x test -x run

# Build WebAssembly
wasm-build:
    wasm-pack build --target web --out-dir pkg

# Pre-commit checks
pre-commit: fmt clippy test
    echo "All checks passed!"
```

---

For more general coding standards that apply to all languages, see [our standards guide.](README.md).
