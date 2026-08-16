<?php

return [
    'title' => 'Today',
    'eyebrow' => 'Command centre',
    'description' => 'A truthful public snapshot of what matters now. Each card keeps its source authority and availability state.',
    'partial' => 'Some public sources are unavailable. Available cards remain visible without inventing missing state.',
    'unavailable' => 'Today cannot currently read any authoritative public source.',
    'cards_label' => 'Today public information cards',
    'source' => 'Open source view',
    'actions' => [
        'view_event' => 'View event',
        'read_news' => 'Read update',
    ],
    'badges' => [
        'featured' => 'Featured',
    ],
    'cards' => [
        'liveops' => [
            'eyebrow' => 'Live world',
            'title' => 'World signals',
            'unavailable' => 'Authoritative LiveOps data is not yet provided to this public surface. No runtime state or recovery status is inferred.',
            'empty' => 'No live world signal.',
            'empty_help' => 'Authoritative LiveOps reported no applicable public signal.',
        ],
        'announcements' => [
            'eyebrow' => 'Notices',
            'title' => 'Announcements',
            'unavailable' => 'Published announcements are temporarily unavailable.',
            'empty' => 'No active announcements.',
            'empty_help' => 'There is no announcement inside its approved publication window.',
        ],
        'events' => [
            'eyebrow' => 'Calendar',
            'title' => 'Next event',
            'unavailable' => 'Event information is temporarily unavailable.',
            'empty' => 'No upcoming event.',
            'empty_help' => 'There is no approved upcoming event for this language.',
        ],
        'news' => [
            'eyebrow' => 'Chronicles',
            'title' => 'Latest news',
            'unavailable' => 'Published news is temporarily unavailable.',
            'empty' => 'No published news.',
            'empty_help' => 'There is no effectively published news for this language.',
        ],
    ],
];
