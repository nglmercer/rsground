mod common;

use common::HELLO_WORLD_OUTPUT;
use rsground_runner::constants::{ECHO, ECHO_NO_NEWLINE};
use rsground_runner::Runner;

#[tokio::test]
async fn echo() {
    let runner = Runner::new().await.expect("The runners was not created");

    let output =
        Runner::collect_output(&mut runner.cmd(ECHO, [ECHO_NO_NEWLINE, HELLO_WORLD_OUTPUT].iter()))
            .await
            .expect("Cannot run code");

    eprintln!("-- STDOUT\n{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("-- STDERR\n{}", String::from_utf8_lossy(&output.stderr));

    assert_eq!(output.status.success(), true);
    assert_eq!(output.stdout, HELLO_WORLD_OUTPUT.as_bytes().to_vec());
}
