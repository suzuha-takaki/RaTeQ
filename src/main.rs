mod cli;
mod render;
mod ui;

use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

fn load(path: &Path) -> Result<String, String> {
    match fs::read(path) {
        Ok(bytes) => {
            if bytes.contains(&0) {
                return Err("E201 error: RaTeQ edits text; binary input is not supported".into());
            }
            let text = String::from_utf8(bytes).map_err(|_| {
                "E201 error: RaTeQ edits UTF-8 text; binary input is not supported".to_string()
            })?;
            Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("error: cannot open {}: {e}", path.display())),
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Internal subprocess keeps expensive rendering cancellable without blocking input.
    if args.first().is_some_and(|a| a == "--internal-render") {
        let width = args
            .get(1)
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&w| w > 0);
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|e| e.to_string())?;
        io::stdout()
            .write_all(&render::png(&source, width)?)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    match cli::parse(&args)? {
        cli::Command::Help => println!("{}", cli::HELP),
        cli::Command::Version => println!("rateq {}", env!("CARGO_PKG_VERSION")),
        cli::Command::Run(options) => {
            if options.resume {
                return Err("E105 error: no saved source file to resume".into());
            }
            let source = options
                .file
                .as_deref()
                .map(load)
                .transpose()?
                .unwrap_or_default();
            ui::run(options, source).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opens_bom_and_rejects_binary() {
        let p = std::env::temp_dir().join(format!("rateq-load-test-{}", std::process::id()));
        fs::write(&p, b"\xef\xbb\xbfx").unwrap();
        assert_eq!(load(&p).unwrap(), "x");
        fs::write(&p, b"a\0b").unwrap();
        assert!(load(&p).unwrap_err().contains("E201"));
        fs::remove_file(p).unwrap();
    }
}
