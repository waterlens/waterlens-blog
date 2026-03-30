#[tokio::main]
async fn main() {
    if let Err(error) = wblog::run().await {
        eprintln!(
            "{} {error:#}",
            wblog::output::tag_stderr("error", wblog::output::RED)
        );
        std::process::exit(1);
    }
}
