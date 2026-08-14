mod ai;
mod app;
mod process;
mod prompt;
mod redaction;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    app::run().await
}
