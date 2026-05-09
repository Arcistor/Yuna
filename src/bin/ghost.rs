#[tokio::main]
async fn main() -> anyhow::Result<()> {
    digital_ghost::app::run_daemon().await
}
