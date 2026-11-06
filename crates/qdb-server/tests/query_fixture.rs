use std::process::Command;

fn qdb() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qdb"))
}

fn fixture() -> String {
    format!("{}/../../tests/fixtures/h1", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn cli_reads_original_table() {
    let output = qdb()
        .arg("query")
        .arg("--data-dir")
        .arg(fixture())
        .arg("--sql")
        .arg("SELECT id, note, location, tag FROM sensors WHERE id > 1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "id\tnote\tlocation\ttag\n2\tNULL\tNULL\tNULL\n"
    );
}

#[test]
fn cli_rejects_unimplemented_statement() {
    let output = qdb()
        .arg("query")
        .arg("--data-dir")
        .arg(fixture())
        .arg("--sql")
        .arg("SELECT 1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unsupported"), "{stderr}");
}
