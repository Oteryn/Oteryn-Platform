<?php

return [

    /*
    |--------------------------------------------------------------------------
    | Character Authority commands (Game `oteryn-game-character-authority-commands-v1`)
    |--------------------------------------------------------------------------
    |
    | Platform stores CreateCharacter / TransferCharacterOwnership intents and Game's receipts;
    | the Game command consumer pulls intents over mTLS. Disabled by default and only honoured in
    | the testing and preproduction environments until the enablement packet. `identities` is a
    | JSON list of dedicated character-command client certificate subjects; a subject shared with
    | any other internal purpose makes the whole surface unavailable.
    |
    */

    'commands' => [
        'enabled' => filter_var(env('CHARACTER_AUTHORITY_COMMANDS_ENABLED', false), FILTER_VALIDATE_BOOLEAN, FILTER_NULL_ON_FAILURE) === true,
        'identities' => env('CHARACTER_AUTHORITY_COMMAND_IDENTITIES'),
        'source_authority' => env('CHARACTER_AUTHORITY_COMMAND_SOURCE_AUTHORITY'),
        'intent_ttl_seconds' => (int) env('CHARACTER_AUTHORITY_COMMAND_INTENT_TTL_SECONDS', 300),
        'max_pending_intents' => (int) env('CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS', 256),
        'requests_per_minute' => (int) env('CHARACTER_AUTHORITY_COMMAND_REQUESTS_PER_MINUTE', 600),
    ],

];
