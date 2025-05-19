use rsground_runner::Runner;

#[tokio::test]
async fn echo() {
    let runner = Runner::new().await.expect("The runners was not created");

    let output = Runner::collect_output(&mut runner.cmd("/bin/echo", ["-n", "Hello World"].iter()))
        .await
        .expect("Cannot run code");

    eprintln!("-- STDOUT\n{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("-- STDERR\n{}", String::from_utf8_lossy(&output.stderr));

    assert_eq!(output.status.success(), true);
    assert_eq!(output.stdout, "Hello World".as_bytes().to_vec());
}
