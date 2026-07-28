<?php

return [
    'header' => [
        ['label' => 'game_catalog.title', 'route' => 'game-catalog.index', 'active' => 'game-catalog.*', 'priority' => 56],
    ],
    'footer' => [
        [
            'key' => 'learn',
            'label' => 'public.wiki.learn',
            'priority' => 25,
            'items' => [
                ['label' => 'game_catalog.items', 'route' => 'game-catalog.items.index', 'active' => 'game-catalog.items.*', 'priority' => 30],
                ['label' => 'game_catalog.creatures', 'route' => 'game-catalog.creatures.index', 'active' => 'game-catalog.creatures.*', 'priority' => 40],
            ],
        ],
    ],
];
