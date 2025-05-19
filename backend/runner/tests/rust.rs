mod common;
use common::{print_output, HELLO_WORLD_RS};
use rsground_runner::Runner;

#[tokio::test]
async fn rust_compilation() {
    let runner = Runner::new().await.expect("The runners was not created");

    runner.create_file("main.rs", HELLO_WORLD_RS).await.unwrap();

    let output = Runner::collect_output(&mut runner.cmd_rustc(["main.rs"]))
        .await
        .expect("Cannot run code");

    print_output(&output);

    assert_eq!(output.status.success(), true);
    assert_eq!(output.stdout, "".as_bytes().to_vec());
}

#[tokio::test]
async fn rust_executable() {
    let runner = Runner::new().await.expect("The runners was not created");

    runner.create_file("main.rs", HELLO_WORLD_RS).await.unwrap();

    let output = Runner::collect_output(&mut runner.cmd_rustc(["main.rs"]))
        .await
        .expect("Cannot run code");

    eprintln!("-- COMPILATION --");
    print_output(&output);

    assert_eq!(output.status.success(), true);

    let output = runner
        .patch_binary("/home/main")
        .await
        .expect("Cannot run code");

    eprintln!("-- PATCHING --");
    print_output(&output);

    assert_eq!(output.status.success(), true);

    let output = Runner::collect_output(&mut runner.cmd("/home/main", [] as [&str; 0]))
        .await
        .expect("Cannot run code");

    eprintln!("-- EXECUTABLE --");
    print_output(&output);

    assert_eq!(output.status.success(), true);
    assert_eq!(output.stdout, "Hello World".as_bytes().to_vec());
}

#[tokio::test]
async fn rust_multi_container_executable() {
    let executer_runner = Runner::new().await.expect("The runners was not created");
    let compiler_runner = Runner::new().await.expect("The runners was not created");

    compiler_runner
        .create_file("main.rs", HELLO_WORLD_RS)
        .await
        .unwrap();

    let output = Runner::collect_output(&mut compiler_runner.cmd_rustc(["main.rs"]))
        .await
        .expect("Cannot run code");

    eprintln!("-- COMPILATION --");
    print_output(&output);

    assert_eq!(output.status.success(), true);

    let output = compiler_runner
        .patch_binary("/home/main")
        .await
        .expect("Cannot run code");

    eprintln!("-- PATCHING --");
    print_output(&output);

    assert_eq!(output.status.success(), true);

    executer_runner
        .copy_file_from_runner(&compiler_runner, "main", "main")
        .await;

    let output = Runner::collect_output(&mut executer_runner.cmd("/home/main", [] as [&str; 0]))
        .await
        .expect("Cannot run code");

    eprintln!("-- EXECUTABLE --");
    print_output(&output);

    assert_eq!(output.status.success(), true);
    assert_eq!(output.stdout, "Hello World".as_bytes().to_vec());
}
