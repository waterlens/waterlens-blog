//! Renders a tree of AsciiDoc files to HTML, for comparing this backend's
//! output against a reference.
//!
//! Usage: `cargo run -p wblog-asciidoc --example render_tree -- <in> <out>`

use std::{env, path::Path, process::ExitCode};

use wblog_asciidoc::{Renderer, adoc_files};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [input_root, output_root] = args.as_slice() else {
        eprintln!("usage: render_tree <input-dir> <output-dir>");
        return ExitCode::FAILURE;
    };

    let input_root = Path::new(input_root);
    let output_root = Path::new(output_root);
    let renderer = Renderer::new();

    let mut failures = 0;
    for input in adoc_files(input_root) {
        let relative = input
            .strip_prefix(input_root)
            .expect("walked paths live under the root");
        let output = output_root.join(relative).with_extension("html");

        match renderer.render_to_file(&input, &output) {
            Ok(warnings) => {
                for warning in warnings {
                    eprintln!("{}: {warning}", input.display());
                }
            }
            Err(error) => {
                eprintln!("{}: {error:#}", input.display());
                failures += 1;
            }
        }
    }

    if failures > 0 {
        eprintln!("{failures} file(s) failed to render");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
