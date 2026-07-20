use std::env;

fn main() {
    if let Err(error) = run() {
        eprintln!("PFTU-01 probe command failed: {error}");
        std::process::exit(1);
    }
}

fn required(value: Option<String>, flag: &str) -> Result<String, String> {
    value.ok_or_else(|| format!("{flag} is required"))
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    let mut database = None;
    let mut input = None;
    let mut output = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--database" => database = args.next(),
            "--input" => input = args.next(),
            "--output" => output = args.next(),
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let value = match command.as_str() {
        "arm" => smalltalk_lib::arm_pftu_01_case_cli(
            required(database, "--database")?,
            required(input, "--input")?,
        )?,
        "export-review" => smalltalk_lib::export_pftu_01_review_cli(
            required(database, "--database")?,
            required(output, "--output")?,
        )?,
        "evaluate" => {
            smalltalk_lib::evaluate_pftu_01_corpus_cli(required(input, "--input")?, output)?
        }
        "help" => {
            print_help();
            return Ok(());
        }
        other => return Err(format!("unknown command {other}")),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn print_help() {
    println!(
        "Usage:\n  pftu_01_probe arm --database FILE --input CASE.json\n  pftu_01_probe export-review --database FILE --output PRIVATE.json\n  pftu_01_probe evaluate --input REDACTED_CORPUS.json [--output REPORT.json]"
    );
}
