use super::*;

fn words(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_owned()).collect()
}

#[test]
fn parses_dry_run_and_actor_and_refuses_a_missing_actor() {
    let actor = Uuid::now_v7();
    let id = actor.to_string();
    assert_eq!(
        parse_args(&words(&["--dry-run", "--actor", &id])),
        Ok(Command {
            dry_run: true,
            actor
        })
    );
    assert_eq!(
        parse_args(&words(&["--actor", &id])),
        Ok(Command {
            dry_run: false,
            actor
        })
    );
    assert_eq!(parse_args(&words(&["--dry-run"])), Err(USAGE));
    assert_eq!(parse_args(&words(&[])), Err(USAGE));
    assert_eq!(parse_args(&words(&["--actor"])), Err(USAGE));
    assert_eq!(
        parse_args(&words(&["--actor", &id, "--actor", &id])),
        Err(USAGE)
    );
    assert_eq!(
        parse_args(&words(&["--dry-run", "--dry-run", "--actor", &id])),
        Err(USAGE)
    );
    assert_eq!(parse_args(&words(&["--actor", "staff"])), Err("Denied"));
    assert_eq!(
        parse_args(&words(&["--actor", &Uuid::nil().to_string()])),
        Err("Denied")
    );
    let refund = DeadRefund {
        operation: actor,
        job: actor,
        state: "pending".into(),
        attempt: None,
    };
    assert_eq!(
        line(&refund, true),
        format!("operation={id} job={id} state=pending attempt=- action=would_revive")
    );
}
