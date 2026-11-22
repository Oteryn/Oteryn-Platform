use std::process::Command;

#[test]
fn migrate_rejects_unexpected_arguments_before_database_configuration()
-> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_oteryn-portal"))
        .env_clear()
        .args(["migrate", "--dry-run"])
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"usage: oteryn-portal migrate\n");
    Ok(())
}
