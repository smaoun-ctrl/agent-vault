use license_secret_agent::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("license_secret_agent=warn")
        .init();

    let cli = Cli::parse();
    cli.run().await
}
