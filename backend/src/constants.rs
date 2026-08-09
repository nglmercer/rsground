//! Stable application values shared by runtime code and integration tests.
//!
//! Keeping protocol names, environment keys, and operational limits here makes
//! changes to the public contract deliberate and keeps tests from duplicating
//! string literals that are easy to mistype.

pub mod env {
    pub const BIND: &str = "RSGROUND_BIND";
    pub const CORS_ORIGINS: &str = "RSGROUND_CORS_ORIGINS";
    pub const ENVIRONMENT: &str = "RSGROUND_ENV";
    pub const MAX_PROJECTS: &str = "RSGROUND_MAX_PROJECTS";
    pub const MAX_USERS: &str = "RSGROUND_MAX_USERS";
    pub const JWT_SECRET: &str = "JWT_SECRET";
    pub const TEST_API_URL: &str = "RSGROUND_TEST_API_URL";
}

pub mod defaults {
    pub const BIND_ADDRESS: &str = "127.0.0.1:8080";
    pub const LOG_FILTER: &str = "info";
    pub const CORS_ORIGINS: [&str; 2] = ["http://localhost:3000", "http://127.0.0.1:3000"];
}

pub mod environment {
    pub const PRODUCTION: &str = "production";
    pub const PRODUCTION_ALIAS: &str = "prod";
}

pub mod http {
    pub const AUTHORIZATION_HEADER: &str = "Authorization";
    pub const CACHE_CONTROL_HEADER: &str = "Cache-Control";
    pub const CONTENT_TYPE_HEADER: &str = "Content-Type";
    pub const LOCATION_HEADER: &str = "Location";
    pub const PROJECT_PASSWORD_HEADER: &str = "X-Project-Password";
    pub const SEC_WEBSOCKET_PROTOCOL_HEADER: &str = "sec-websocket-protocol";
    pub const CACHE_CONTROL_NO_STORE: &str = "no-store";
    pub const BEARER_PREFIX: &str = "Bearer ";
    pub const GET_METHOD: &str = "GET";
    pub const POST_METHOD: &str = "POST";
    pub const OPTIONS_METHOD: &str = "OPTIONS";
    pub const CORS_WILDCARD: &str = "*";
}

pub mod url {
    pub const HTTP_SCHEME: &str = "http://";
    pub const HTTPS_SCHEME: &str = "https://";
    pub const WS_SCHEME: &str = "ws://";
    pub const WSS_SCHEME: &str = "wss://";
}

pub mod limits {
    pub const DEFAULT_MAX_PROJECTS: usize = 64;
    pub const DEFAULT_MAX_USERS: usize = 10_000;
    pub const MAX_PROJECT_FILES: usize = 256;
    pub const MAX_PROJECT_NAME_CHARS: usize = 128;
    pub const MAX_PROJECT_PASSWORD_BYTES: usize = 256;
    pub const MAX_FILE_PATH_BYTES: usize = 512;
    pub const MAX_GUEST_NAME_CHARS: usize = 64;
    pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;
}

pub mod collaboration {
    pub const INITIAL_REVISION: usize = 0;
}

pub mod filesystem {
    pub const NUL_BYTE: u8 = 0;
}

pub mod auth {
    pub const OAUTH_STATE_COOKIE: &str = "rsground_oauth_state";
    pub const OAUTH_COOKIE_PATH: &str = "/auth";
    pub const OAUTH_SCOPE_READ_USER: &str = "read:user";
    pub const OAUTH_COOKIE_MAX_AGE_MINUTES: i64 = 10;
    pub const JWT_EXPIRATION_HOURS: i64 = 12;
    pub const MIN_DEPLOYMENT_SECRET_BYTES: usize = 32;
}

pub mod project {
    pub const MAIN_FILE: &str = "main.rs";
    pub const ANALYZER_CARGO_FILE: &str = "Cargo.toml";
    pub const ANALYZER_CARGO_SOURCE: &str = r#"[package]
name = "rsground-playground"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "main"
path = "main.rs"
"#;
    pub const MAIN_EXECUTABLE: &str = "main";
    pub const RUNNER_MAIN_FILE: &str = "/home/main.rs";
    pub const RUNNER_MAIN_EXECUTABLE: &str = "/home/main";
    pub const RUSTC_COLOR_ARGUMENT: &str = "--color";
    pub const RUSTC_COLOR_ALWAYS: &str = "always";
    pub const MAIN_SOURCE: &str = r#"fn main() {
    println!("Hello World");
}"#;
    pub const FORK_SUFFIX: &str = " (fork)";
}

pub mod websocket {
    pub const AUTH_PROTOCOL: &str = "auth";
    pub const PASSWORD_PROTOCOL: &str = "password";
    pub const PING: &str = "ping";
    pub const HEARTBEAT_INTERVAL_SECS: u64 = 5;
    pub const FILE_READ_CONCURRENCY: usize = 5;
    pub const MAX_CONTINUATION_SIZE_BYTES: usize = 1 << 20;
    pub const DOCUMENT_CONCURRENCY: usize = 10;
    pub const BROADCAST_CAPACITY: usize = u8::MAX as usize;
}

pub mod output {
    pub const BUFFER_SIZE: usize = 2048;
    pub const COMPILE_FAILURE_EXIT_CODE: u8 = 126;
}

pub mod json {
    pub const ACTION: &str = "action";
    pub const ACCESS: &str = "access";
    pub const AVATAR_URL: &str = "avatar_url";
    pub const ERROR: &str = "error";
    pub const FILE: &str = "file";
    pub const FILES: &str = "files";
    pub const GUEST_NAME: &str = "guest_name";
    pub const HAS_PASSWORD: &str = "has_password";
    pub const ID: &str = "id";
    pub const IS_GUEST: &str = "is_guest";
    pub const IS_OWNER: &str = "is_owner";
    pub const IS_PUBLIC: &str = "is_public";
    pub const ACTIONS: &str = "actions";
    pub const BUF: &str = "buf";
    pub const CHANNEL: &str = "channel";
    pub const JWT: &str = "jwt";
    pub const MESSAGE: &str = "message";
    pub const NAME: &str = "name";
    pub const OWNER: &str = "owner";
    pub const PASSWORD: &str = "password";
    pub const REVISION: &str = "revision";
    pub const REQUESTS: &str = "requests";
    pub const SESSION_ID: &str = "session_id";
    pub const USER_ID: &str = "user_id";
    pub const USERS: &str = "users";
}

pub mod output_channel {
    pub const STDOUT: &str = "stdout";
    pub const STDERR: &str = "stderr";
}

pub mod access {
    pub const EDITOR: &str = "editor";
}

/// Serialized WebSocket action names. The server derives these values from
/// the message enum, while clients and integration tests use this table when
/// constructing raw JSON messages.
pub mod ws_action {
    pub const CONFIG: &str = "config";
    pub const ERROR: &str = "error";
    pub const EXECUTE: &str = "execute";
    pub const FILE_CREATE: &str = "file_create";
    pub const FILE_DELETE: &str = "file_delete";
    pub const LSP: &str = "lsp";
    pub const PERMIT_ACCESS: &str = "permit_access";
    pub const PROJECT_CONFIG: &str = "project_config";
    pub const PROJECT_FILES: &str = "project_files";
    pub const REQUEST_ACCESS: &str = "request_access";
    pub const STOP_EXECUTE: &str = "stop_execute";
    pub const SYNC: &str = "sync";
    pub const SYNC_OUTPUT: &str = "sync_output";
    pub const SYNC_OUTPUT_END: &str = "sync_output_end";
    pub const SYNC_OUTPUT_START: &str = "sync_output_start";
    pub const SYNC_CURSORS: &str = "sync_cursors";
    pub const SYNC_FILES: &str = "sync_files";
    pub const UPDATE_ACCESS: &str = "update_access";
    pub const USER_CONNECTED: &str = "user_connected";
    pub const WELCOME: &str = "welcome";
}

pub mod route {
    pub const HEALTH: &str = "/health";
    pub const WS_BASE: &str = "/ws";
    pub const AUTH: &str = "/auth";
    pub const AUTH_CALLBACK: &str = "/auth/callback";
    pub const AUTH_GUEST: &str = "/auth/guest";
    pub const AUTH_ME: &str = "/auth/me";
    pub const AUTH_UPDATE: &str = "/auth/update";
    pub const PROJECT_CREATE: &str = "/create/test_project";
    pub const PROJECT: &str = "/project";
}
