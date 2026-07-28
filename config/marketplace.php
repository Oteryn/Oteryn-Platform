<?php

return [
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
