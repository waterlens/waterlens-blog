use anyhow::{Result, anyhow};
use live_server::{Options, listen};

use crate::cli::ServeArgs;
use crate::output;

pub async fn serve(args: ServeArgs) -> Result<()> {
    let address = format!("{}:{}", args.host, args.port);
    let root = args.root.to_string_lossy().into_owned();
    println!(
        "{} {} {}",
        output::tag_stdout("serve", output::GREEN),
        output::accent_stdout(&root, output::DIM),
        output::accent_stdout(&display_url(&args.host, args.port), output::CYAN),
    );
    let listener = listen(address, root)
        .await
        .map_err(|error| anyhow!(error))?;
    listener
        .start(Options::default())
        .await
        .map_err(|error| anyhow!(error.to_string()))?;
    Ok(())
}

fn display_url(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("http://[{host}]:{port}")
    } else {
        format!("http://{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::display_url;

    #[test]
    fn display_url_formats_ipv4() {
        assert_eq!(display_url("127.0.0.1", 8668), "http://127.0.0.1:8668");
    }

    #[test]
    fn display_url_formats_ipv6() {
        assert_eq!(display_url("::1", 8668), "http://[::1]:8668");
    }
}
