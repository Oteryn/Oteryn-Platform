#[test]
fn r7_stop_writers_before_migrate() {
    let script = include_str!("../../../../deploy/oracle/prepare-platform.sh");
    let commands: Vec<_> = script
        .lines()
        .filter_map(|line| line.strip_prefix("\"${compose[@]}\" "))
        .collect();
    assert!(
        commands.windows(3).any(|w| w
            == [
                "stop platform worker",
                "run --rm -T --no-deps migrate",
                "up -d --no-deps platform worker gateway"
            ]),
        "migration must run between stop and start"
    );
}
