<?php

return [
    'header' => [
        ['label' => 'Home', 'route' => 'home', 'active' => 'home', 'priority' => 10],
        ['label' => 'Today', 'route' => 'today.index', 'active' => 'today.*', 'priority' => 15],
        ['label' => 'News', 'route' => 'news.index', 'active' => 'news.*', 'priority' => 20],
        ['label' => 'Online', 'route' => 'game.online.index', 'active' => 'game.online.*', 'priority' => 30],
        ['label' => 'Highscores', 'route' => 'game.highscores.index', 'active' => 'game.highscores.*', 'priority' => 40],
        ['label' => 'Servers', 'route' => 'game.servers.index', 'active' => 'game.servers.*', 'priority' => 50],
    ],
    'footer' => [
        [
            'key' => 'world',
            'label' => 'World',
            'priority' => 10,
            'items' => [
                ['label' => 'Online', 'route' => 'game.online.index', 'active' => 'game.online.*', 'priority' => 10],
                ['label' => 'Highscores', 'route' => 'game.highscores.index', 'active' => 'game.highscores.*', 'priority' => 20],
                ['label' => 'Servers', 'route' => 'game.servers.index', 'active' => 'game.servers.*', 'priority' => 30],
                ['label' => 'Character search', 'route' => 'home', 'fragment' => 'character-search', 'active' => 'game.characters.*', 'priority' => 40],
            ],
        ],
        [
            'key' => 'chronicles',
            'label' => 'Chronicles',
            'priority' => 20,
            'items' => [
                ['label' => 'Today', 'route' => 'today.index', 'active' => 'today.*', 'priority' => 5],
                ['label' => 'Latest news', 'route' => 'news.index', 'active' => 'news.*', 'priority' => 10],
            ],
        ],
    ],
];
