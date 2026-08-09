use common::cargo::cargo_init;
use common::print_output;
use rsground_runner::constants::{
    BUILD, BUILT_BINARY, BUILT_BINARY_RELATIVE, CARGO, MAIN_EXECUTABLE, RELEASE,
    RUNNER_MAIN_EXECUTABLE,
};
use rsground_runner::Runner;

mod common;

#[tokio::test]
async fn cargo_build() {
    let runner = Runner::new().await.unwrap();
    cargo_init(&runner).await;

    let output = Runner::collect_output(&mut runner.cmd(CARGO, [BUILD, RELEASE]))
        .await
        .unwrap();

    print_output(&output);

    assert_eq!(output.status.success(), true);
}

#[tokio::test]
async fn cargo_run() {
    let runner = Runner::new().await.unwrap();
    cargo_init(&runner).await;

    let output = Runner::collect_output(&mut runner.cmd(CARGO, [BUILD, RELEASE]))
        .await
        .unwrap();

    print_output(&output);
    assert_eq!(output.status.success(), true);

    let output = runner.patch_binary(BUILT_BINARY).await.unwrap();

    print_output(&output);
    assert_eq!(output.status.success(), true);

    let executer_runner = Runner::new().await.unwrap();
    executer_runner
        .copy_file_from_runner(&runner, MAIN_EXECUTABLE, BUILT_BINARY_RELATIVE)
        .await
        .expect("Cannot copy executable");

    let output =
        Runner::collect_output(&mut executer_runner.cmd(RUNNER_MAIN_EXECUTABLE, [] as [&str; 0]))
            .await
            .unwrap();

    print_output(&output);
    assert_eq!(output.status.success(), true);
    assert_eq!(
        output.stdout,
        common::HELLO_WORLD_OUTPUT.as_bytes().to_vec()
    );
}
