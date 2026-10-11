<?php

$environment = env('APP_ENV', 'production');
$isolatedTestDefault = in_array($environment, ['testing', 'acceptance'], true);

return [
    // Runtime environments are fail-closed. The isolated PHPUnit/acceptance
    // harnesses retain their deterministic fixture default. Every other runtime
    // requires the reviewed escrow, transfer credential, scheduler and control gate.
    // Trusted-main staging refreshes still execute through that guarded control.
    'enabled' => (bool) env('MARKETPLACE_ENABLED', $isolatedTestDefault),
    // Bazaar freeze (removal plan B4): new listings are refused while true. In-flight
    // auctions keep settling through marketplace:reconcile; marketplace:drain
    // reports when none are left. Fail-closed outside the isolated test harnesses.
    'listing_frozen' => (bool) env('MARKETPLACE_LISTING_FROZEN', ! $isolatedTestDefault),
    'currency_name' => 'Oteryn Coins',
    'escrow_canary_account_id' => (int) env('MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID', 0),
    'allowed_duration_days' => [1, 3, 7],
    'minimum_starting_bid' => 100,
    'minimum_bid_increment' => 10,
    'commission_basis_points' => 1000,
    'escrow_quiescence_seconds' => 30,
    'public_bid_history_limit' => 20,
    'character_limit' => 10,
];
