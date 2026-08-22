<?php

if (! config('marketplace.enabled')) {
    return ['header' => [], 'footer' => []];
}

return [
    'header' => [
        ['label' => 'Character Bazaar', 'route' => 'marketplace.index', 'active' => 'marketplace.*', 'priority' => 45],
    ],
    'footer' => [
        [
            'key' => 'community',
            'label' => 'public.game.community',
            'priority' => 30,
            'items' => [
                ['label' => 'Character Bazaar', 'route' => 'marketplace.index', 'active' => 'marketplace.*', 'priority' => 25],
            ],
        ],
    ],
];
