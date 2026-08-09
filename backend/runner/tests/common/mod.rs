#![allow(
    dead_code,
    reason = "Common is used in many test, there are dead_code for some of them"
)]

pub mod cargo;

use hakoniwa::Output;

pub const HELLO_WORLD_RS: &str = r#"
fn main() {
    print!("Hello World");
}
"#;
pub const HELLO_WORLD_OUTPUT: &str = "Hello World";
pub const JSON_RPC_VERSION: &str = "2.0";
pub const JSON_RPC_CONTENT_LENGTH: &str = "Content-Length";
pub const JSON_RPC_HEADER_SEPARATOR: &str = "\r\n\r\n";
pub const INITIAL_REQUEST_ID: u16 = 1;
pub const INITIALIZE_METHOD: &str = "initialize";
pub const INITIALIZE_PARAMS: &str = r#"{"capabilities": {}}"#;
pub const INITIALIZED_METHOD: &str = "initialized";
pub const EXIT_METHOD: &str = "exit";
pub const EMPTY_JSON_OBJECT: &str = "{}";
pub const RUST_ANALYZER_IO_DELAY_MS: u64 = 10;
pub const RUST_ANALYZER_BUFFER_SIZE: usize = 1024;

pub fn print_output(output: &Output) {
    eprintln!("-- STDOUT\n{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("-- STDERR\n{}", String::from_utf8_lossy(&output.stderr));
}
