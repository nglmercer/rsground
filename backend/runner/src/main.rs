use rsground_runner::constants;

#[tokio::main]
async fn main() {
    let runner = rsground_runner::Runner::new().await.unwrap();

    runner
        .create_file(
            constants::MAIN_FILE,
            r#"
            fn main() {
                println!("Hello World");
            }
            "#,
        )
        .await
        .unwrap();

    runner
        .create_file(constants::C_MAIN_FILE, constants::C_MAIN_SOURCE)
        .await
        .unwrap();

    let mut cmd = runner
        .spawn(constants::BASH, [constants::BASH_INTERACTIVE])
        .unwrap();

    cmd.wait().unwrap();
}
