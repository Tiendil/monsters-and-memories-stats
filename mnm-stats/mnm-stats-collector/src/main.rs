use mnm_stats_model::History;
use std::{env, error::Error, fs, process::ExitCode};

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let command = args.next();
    let path = args.next();
    if command.as_deref() != Some(std::ffi::OsStr::new("validate-history"))
        || path.is_none()
        || args.next().is_some()
    {
        return Err("usage: mnm-stats-collector validate-history PATH".into());
    }
    let history = History::from_jsonl(&fs::read_to_string(path.unwrap())?)?;
    println!("Valid history: {} observations", history.snapshots().len());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
