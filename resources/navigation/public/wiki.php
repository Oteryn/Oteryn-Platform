<?php

return [
    'header' => [
        ['label' => 'public.wiki.title', 'route' => 'wiki.index', 'active' => 'wiki.*', 'priority' => 55],
    ],
    'footer' => [
        [
            'key' => 'learn',
            'label' => 'public.wiki.learn',
            'priority' => 25,
            'items' => [
                ['label' => 'public.wiki.title', 'route' => 'wiki.index', 'active' => 'wiki.*', 'priority' => 10],
                ['label' => 'public.wiki.search_title', 'route' => 'wiki.search', 'active' => 'wiki.search', 'priority' => 20],
            ],
        ],
    ],
];
