<?php

return [
    'header' => [
        ['label' => 'Character Bazaar', 'route' => 'marketplace.index', 'active' => 'marketplace.*', 'priority' => 45],
    ],
    'footer' => [
        [
            'key' => 'community',
            'label' => 'Community',
            'priority' => 30,
            'items' => [
                ['label' => 'Character Bazaar', 'route' => 'marketplace.index', 'active' => 'marketplace.*', 'priority' => 25],
            ],
        ],
    ],
];
