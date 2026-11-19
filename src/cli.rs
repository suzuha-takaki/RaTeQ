use std::path::PathBuf;

pub const HELP: &str = "Usage: rateq [OPTIONS] [FILE]\n\nOpen and edit a LaTeX math fragment.\n\nOptions:\n  -r, --resume   Reopen the last source file saved through RaTeQ\n  -l, --live     Render after 300 ms without source edits\n  -s, --single   Show one window at a time\n  -b, --break    Break long equations into preview lines\n  -h, --help     Show CLI help\n  -v, --version  Show version\n\nWith no options, start an unnamed source in manual, double-window mode.\nLong equations scroll by default; --break enables preview line breaks.\nEnter renders immediately in manual and live modes.\nLive and single-window modes cannot be combined.\nResume cannot be combined with FILE.\nRun help on its own with rateq -h, or version on its own with rateq -v.";
#[derive(Default, Debug)]
pub struct Options {
    pub live: bool,
    pub single: bool,
    pub breaking: bool,
    pub resume: bool,
    pub file: Option<PathBuf>,
}
pub enum Command {
    Run(Options),
    Help,
    Version,
}

pub fn parse(args: &[String]) -> Result<Command, String> {
    let is_short =
        |s: &str, c: char| s.starts_with('-') && !s.starts_with("--") && s[1..].contains(c);
    let help = args
        .iter()
        .take_while(|s| s.as_str() != "--")
        .any(|s| s == "--help" || is_short(s, 'h'));
    let version = args
        .iter()
        .take_while(|s| s.as_str() != "--")
        .any(|s| s == "--version" || is_short(s, 'v'));
    if help || version {
        if args.len() == 1 && matches!(args[0].as_str(), "-h" | "--help" | "-v" | "--version") {
            return Ok(if help {
                Command::Help
            } else {
                Command::Version
            });
        }
        let hint = match (help, version) {
            (true, true) => "'rateq -h' or 'rateq -v'",
            (true, false) => "'rateq -h'",
            _ => "'rateq -v'",
        };
        return Err(format!(
            "E106 error: help and version must each be run on their own\nDid you mean {hint}?"
        ));
    }
    let mut opt = Options::default();
    let mut positional = false;
    for arg in args {
        if positional {
            if opt.file.replace(arg.into()).is_some() {
                return Err("E104 error: only one filename may be provided".into());
            }
            continue;
        }
        if arg == "--" {
            positional = true;
            continue;
        }
        let flags: Vec<String> = if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2
        {
            arg[1..].chars().map(|c| format!("-{c}")).collect()
        } else {
            vec![arg.clone()]
        };
        for flag in flags {
            match flag.as_str() {
                "-l" | "--live" => opt.live = true,
                "-s" | "--single" => opt.single = true,
                "-b" | "--break" => opt.breaking = true,
                "-r" | "--resume" => opt.resume = true,
                s if s.starts_with('-') && s != "-" => {
                    return Err(format!("E103 error: unknown option {s}"))
                }
                _ => {
                    if opt.file.replace(PathBuf::from(&flag)).is_some() {
                        return Err("E104 error: only one filename may be provided".into());
                    }
                }
            }
        }
    }
    if opt.live && opt.single {
        return Err("E101 error: --live (-l) cannot be combined with --single (-s)".into());
    }
    if opt.resume && opt.file.is_some() {
        return Err("E102 error: --resume (-r) cannot be combined with a filename".into());
    }
    Ok(Command::Run(opt))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn validates_priority_and_modes() {
        assert!(parse(&args(&["--help", "--bad"]))
            .err()
            .unwrap()
            .contains("E106"));
        assert!(parse(&args(&["-ls"])).err().unwrap().contains("E101"));
        assert!(parse(&args(&["-r", "x"])).err().unwrap().contains("E102"));
        assert!(parse(&args(&["x", "y"])).err().unwrap().contains("E104"));
    }
}
