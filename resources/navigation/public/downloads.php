<?php

return [
    'header' => [
        ['label' => 'public.navigation.download', 'route' => 'downloads.index', 'active' => 'downloads.*', 'priority' => 80],
    ],
    'footer' => [
        [
            'key' => 'learn',
            'label' => 'public.wiki.learn',
            'priority' => 25,
            'items' => [
                ['label' => 'public.navigation.download', 'route' => 'downloads.index', 'active' => 'downloads.*', 'priority' => 5],
            ],
        ],
    ],
];
