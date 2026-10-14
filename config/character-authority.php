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
        'intent_ttl_seconds' => env('CHARACTER_AUTHORITY_COMMAND_INTENT_TTL_SECONDS', 300),
        'max_pending_intents' => env('CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS', 256),
        'requests_per_minute' => env('CHARACTER_AUTHORITY_COMMAND_REQUESTS_PER_MINUTE', 600),
    ],

    /*
    |--------------------------------------------------------------------------
    | Web character creation (Game contract §14 step 3)
    |--------------------------------------------------------------------------
    |
    | When on, and the command surface above is usable, the Account Center form records a
    | `CreateCharacter` intent and shows the outcome from Game's receipt. Off by default: the form
    | keeps answering "temporarily unavailable". Enablement (step 4) waits on U-CC7 and U-CC9.
    |
    */

    'create' => [
        'enabled' => filter_var(env('CHARACTER_AUTHORITY_WEB_CREATE_ENABLED', false), FILTER_VALIDATE_BOOLEAN, FILTER_NULL_ON_FAILURE) === true,
    ],

];
