#[tokio::main]
async fn main() -> anyhow::Result<()> {
    yuna::app::run_daemon().await
}
