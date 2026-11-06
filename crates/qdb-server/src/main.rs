use qdb_sql::{execute, render_tsv};
use qdb_types::Error;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            if err.message.starts_with("usage:") {
                ExitCode::from(2)
            } else {
                ExitCode::from(1)
            }
        }
    }
}

fn run(args: Vec<String>) -> Result<(), Error> {
    if args.first().map(String::as_str) != Some("query") {
        return Err(Error::new(
            "usage: qdb query --data-dir DIR --sql SQL\n\
             reads a PARTITION BY NONE table and prints a tab-separated result",
        ));
    }
    let mut data_dir: Option<PathBuf> = None;
    let mut sql: Option<String> = None;
    let mut index = 1;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| Error::new(format!("missing value for {flag}")))?;
        match flag {
            "--data-dir" => data_dir = Some(PathBuf::from(value)),
            "--sql" => sql = Some(value.clone()),
            other => return Err(Error::new(format!("unknown argument `{other}`"))),
        }
        index += 2;
    }
    let data_dir = data_dir.ok_or_else(|| Error::new("missing --data-dir"))?;
    let sql = sql.ok_or_else(|| Error::new("missing --sql"))?;
    let result = execute(&data_dir, &sql)?;
    print!("{}", render_tsv(&result));
    Ok(())
}
