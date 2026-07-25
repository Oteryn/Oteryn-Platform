<?php

return [
    'header' => [
        ['label' => 'Events', 'route' => 'events.index', 'active' => 'events.*', 'priority' => 55],
    ],
    'footer' => [
        [
            'key' => 'chronicles',
            'label' => 'Chronicles',
            'priority' => 20,
            'items' => [
                ['label' => 'Events', 'route' => 'events.index', 'active' => 'events.*', 'priority' => 20],
            ],
        ],
    ],
];
