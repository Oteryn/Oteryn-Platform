<?php

return [
    /*
    | Legacy game-data source switch for the pages that still read the obsolete connection
    | (profile, guild, deaths, servers). The online and highscores pages never read it: they
    | are served from the native public projections only.
    */
    'source' => env('PUBLIC_GAME_DATA_SOURCE'),

    /*
    | The one World the online and highscores pages serve (a Game world id, UUID). Empty means
    | the only World that has published; with several Worlds and no value the pages fail closed.
    */
    'world_id' => env('PUBLIC_GAME_DATA_WORLD_ID'),
];
