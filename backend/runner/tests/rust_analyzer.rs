#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused_imports, reason = "this file no test in debug mode")
)]
mod common;

use std::io::{Read, Write};
use std::time::Duration;

use common::cargo::cargo_init;
use common::{
    EMPTY_JSON_OBJECT, EXIT_METHOD, INITIALIZED_METHOD, INITIALIZE_METHOD, INITIALIZE_PARAMS,
    INITIAL_REQUEST_ID, JSON_RPC_CONTENT_LENGTH, JSON_RPC_HEADER_SEPARATOR, JSON_RPC_VERSION,
    RUST_ANALYZER_BUFFER_SIZE, RUST_ANALYZER_IO_DELAY_MS,
};
use rsground_runner::Runner;

fn make_request(id: u16, method: &str, params: &str) -> String {
    let content = format!(
        r#"{{"jsonrpc":"{JSON_RPC_VERSION}","id":{id},"method":"{method}","params":{params}}}"#
    );
    format!(
        "{JSON_RPC_CONTENT_LENGTH}: {}{JSON_RPC_HEADER_SEPARATOR}{content}",
        content.len()
    )
}

fn make_notify(method: &str, params: &str) -> String {
    let content =
        format!(r#"{{"jsonrpc":"{JSON_RPC_VERSION}","method":"{method}","params":{params}}}"#);
    format!(
        "{JSON_RPC_CONTENT_LENGTH}: {}{JSON_RPC_HEADER_SEPARATOR}{content}",
        content.len()
    )
}

/// Only test in release mode, this is a slow test
#[cfg(not(debug_assertions))]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rust_analyzer_start() {
    let mut runner = Runner::new().await.unwrap();
    cargo_init(&runner).await;

    let (mut child, mut stdin, mut stdout, mut stderr) = runner.start_rls().unwrap();

    let a = tokio::spawn(async move {
        let content = make_request(INITIAL_REQUEST_ID, INITIALIZE_METHOD, INITIALIZE_PARAMS);
        println!("[STDIN] {content}");
        stdin.write_all(content.as_bytes()).unwrap();

        _ = tokio::time::sleep(Duration::from_millis(RUST_ANALYZER_IO_DELAY_MS));

        let content = make_notify(INITIALIZED_METHOD, EMPTY_JSON_OBJECT);
        println!("[STDIN] {content}");
        stdin.write_all(content.as_bytes()).unwrap();

        _ = tokio::time::sleep(Duration::from_millis(RUST_ANALYZER_IO_DELAY_MS));

        let content = make_notify(EXIT_METHOD, EMPTY_JSON_OBJECT);
        println!("[STDIN] {content}");
        stdin.write_all(content.as_bytes()).unwrap();
    });

    let b = tokio::spawn(async move {
        let buf = &mut [0u8; RUST_ANALYZER_BUFFER_SIZE];
        loop {
            let size = stdout.read(buf).unwrap();
            if size == 0 {
                break;
            }

            println!("[STDOUT]: {}", String::from_utf8_lossy(&buf[..size]));
        }
    });

    let c = tokio::spawn(async move {
        let buf = &mut [0u8; RUST_ANALYZER_BUFFER_SIZE];
        loop {
            let size = stderr.read(buf).unwrap();
            if size == 0 {
                break;
            }

            println!("[STDERR]: {}", String::from_utf8_lossy(&buf[..size]));
        }
    });

    _ = tokio::join!(a, b, c);
    let exit = child.wait().expect("Cannot wait, now, now");
    println!("{exit:#?}");

    assert!(exit.success())
}
