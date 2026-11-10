#[path = "../pr_guide/mod.rs"]
mod pr_guide;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pr_guide::run().await
}
