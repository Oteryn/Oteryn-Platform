<?php

$environment = env('APP_ENV', 'production');
$isolatedTestDefault = in_array($environment, ['testing', 'acceptance'], true);

return [
    // Runtime environments are fail-closed. The isolated PHPUnit/acceptance
    // harnesses retain their deterministic fixture default. Every other runtime
    // requires the reviewed escrow, transfer credential, scheduler and control gate.
    'enabled' => (bool) env('MARKETPLACE_ENABLED', $isolatedTestDefault),
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
