<?php

return [
    'header' => [
        ['label' => 'public.navigation.guilds', 'route' => 'game.guilds.index', 'active' => 'game.guilds.*', 'priority' => 45],
        ['label' => 'community.navigation.deaths', 'route' => 'game.deaths.index', 'active' => 'game.deaths.*', 'priority' => 46],
    ],
    'footer' => [
        [
            'key' => 'world',
            'label' => 'World',
            'priority' => 10,
            'items' => [
                ['label' => 'public.navigation.guilds', 'route' => 'game.guilds.index', 'active' => 'game.guilds.*', 'priority' => 25],
                ['label' => 'community.navigation.deaths', 'route' => 'game.deaths.index', 'active' => 'game.deaths.*', 'priority' => 26],
            ],
        ],
    ],
];
