<?php

return [
    'header' => [
        ['label' => 'Beginner\'s Guide', 'route' => 'editorial.getting-started', 'active' => 'editorial.getting-started', 'priority' => 55],
        ['label' => 'Support', 'route' => 'support.index', 'active' => 'support.*', 'priority' => 70],
    ],
    'footer' => [
        [
            'key' => 'learn',
            'label' => 'Learn',
            'priority' => 30,
            'items' => [
                ['label' => 'Beginner\'s Guide', 'route' => 'editorial.getting-started', 'active' => 'editorial.getting-started', 'priority' => 10],
                ['label' => 'Server Information', 'route' => 'editorial.server-information', 'active' => 'editorial.server-information', 'priority' => 20],
                ['label' => 'Rules', 'route' => 'editorial.rules', 'active' => 'editorial.rules', 'priority' => 30],
            ],
        ],
        [
            'key' => 'support-legal',
            'label' => 'Support and legal',
            'priority' => 40,
            'items' => [
                ['label' => 'Support', 'route' => 'support.index', 'active' => 'support.index', 'priority' => 10],
                ['label' => 'Report a Bug', 'route' => 'support.report-a-bug', 'active' => 'support.report-a-bug', 'priority' => 20],
                ['label' => 'Terms of Service', 'route' => 'legal.terms', 'active' => 'legal.terms', 'priority' => 30],
                ['label' => 'Privacy Policy', 'route' => 'legal.privacy', 'active' => 'legal.privacy', 'priority' => 40],
                ['label' => 'Cookie Policy', 'route' => 'legal.cookies', 'active' => 'legal.cookies', 'priority' => 50],
            ],
        ],
    ],
];
