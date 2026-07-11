use std::env;

fn main() {
    if let Err(error) = run() {
        eprintln!("continue accuracy eval failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut fixture_root = None;
    let mut output_path = None;
    let mut allow_locked_holdout = false;
    let mut repeat_count = 2usize;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--fixtures" => {
                fixture_root = Some(
                    arguments
                        .next()
                        .ok_or_else(|| "--fixtures requires a directory".to_string())?,
                );
            }
            "--output" => {
                output_path = Some(
                    arguments
                        .next()
                        .ok_or_else(|| "--output requires a path".to_string())?,
                );
            }
            "--repeat" => {
                repeat_count = arguments
                    .next()
                    .ok_or_else(|| "--repeat requires an integer".to_string())?
                    .parse::<usize>()
                    .map_err(|_| "--repeat must be an integer".to_string())?;
            }
            "--allow-locked-holdout" => allow_locked_holdout = true,
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --bin continue_accuracy_eval -- \
                     [--fixtures DIR] [--output FILE] [--repeat N] \
                     [--allow-locked-holdout]"
                );
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let report = smalltalk_lib::run_continue_accuracy_eval_cli(
        fixture_root,
        output_path,
        allow_locked_holdout,
        repeat_count,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(())
}
