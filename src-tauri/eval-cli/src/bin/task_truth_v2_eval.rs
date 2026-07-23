use std::env;

fn main() {
    if let Err(error) = run() {
        eprintln!("Task Truth v2 command failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1).peekable();
    let command = if arguments.peek().is_some_and(|value| value == "build") {
        arguments.next();
        "build"
    } else {
        "eval"
    };
    let mut input = None;
    let mut fixtures = None;
    let mut output = None;
    let mut allow_locked_holdout = false;
    let mut write = false;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--input" => input = Some(arguments.next().ok_or("--input requires a file")?),
            "--fixtures" => {
                fixtures = Some(arguments.next().ok_or("--fixtures requires a directory")?)
            }
            "--output" => output = Some(arguments.next().ok_or("--output requires a file")?),
            "--allow-locked-holdout" => allow_locked_holdout = true,
            "--write" => write = true,
            "--dry-run" => write = false,
            "--help" | "-h" => {
                println!("Usage:\n  task_truth_v2_eval [--fixtures DIR] [--output FILE] [--allow-locked-holdout]\n  task_truth_v2_eval build --input FILE [--dry-run | --write --output FILE]");
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let value = if command == "build" {
        smalltalk_lib::build_task_truth_v2_candidate_cli(
            input.ok_or("build requires --input")?,
            output,
            !write,
        )?
    } else {
        smalltalk_lib::run_task_truth_v2_eval_cli(fixtures, output, allow_locked_holdout)?
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
    );
    Ok(())
}
