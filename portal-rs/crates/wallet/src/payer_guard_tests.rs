//! Every new player debit from the wallet's available balance must reach the
//! locked payer check. Administrative and provider adjustments are distinct.
#[test]
fn player_spend_paths_check_the_locked_payer() {
    let ledger = include_str!("ledger.rs");
    let mut guarded = Vec::new();
    for part in ledger.split("async fn ").skip(1) {
        let function = part.split('(').next().unwrap_or_default();
        let debit = part.find("entry(").filter(|_| {
            part.contains("sender, -amount") || part.contains("account, -amount, amount")
        });
        let purchase_adjustment = function == "adjust_in";
        if debit.is_none() && !purchase_adjustment {
            continue;
        }
        let check = part.find("check_payer_in(").unwrap_or(part.len());
        let effect = debit.or_else(|| part.find("credit(tx, key, account, amount)"));
        assert!(effect.is_some_and(|effect| check < effect), "{function}");
        assert!(
            part[..check].contains("claim("),
            "{function} must allow replay"
        );
        guarded.push(function);
    }
    assert_eq!(
        guarded,
        ["transfer_inner", "adjust_in", "reserve_in", "reserve_offer"]
    );
    let reserve = ledger
        .split("async fn reserve(")
        .nth(1)
        .and_then(|body| body.split("async fn ").next())
        .unwrap_or_default();
    assert!(reserve.contains("self.reserve_in(&mut tx, key, account, amount)"));
    let adjustment = ledger
        .split("async fn adjust_in(")
        .nth(1)
        .unwrap_or_default();
    assert!(adjustment.contains("if self.adjustment_kind.is_purchase()"));
}
