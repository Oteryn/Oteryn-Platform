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
            'partial' => 'Some world evidence is stale, unavailable, invalid or mixed. Last-known facts remain explicitly qualified.',
            'states' => [
                'maintenance' => ['label' => 'Maintenance', 'summary' => 'Platform maintenance is configured for this world. Runtime readiness cannot override the maintenance gate.'],
                'policy_offline' => ['label' => 'Configured offline', 'summary' => 'Platform policy currently marks this world unavailable for new entry; this is not inferred from runtime failure.'],
                'login_disabled' => ['label' => 'Login disabled', 'summary' => 'Platform policy currently denies new entry for this world.'],
                'policy_unknown' => ['label' => 'Policy unknown', 'summary' => 'Platform policy cannot currently confirm new-entry availability.'],
                'ready' => ['label' => 'Ready', 'summary' => 'Fresh authoritative runtime evidence reports every current channel ready.'],
                'not_ready' => ['label' => 'Not ready', 'summary' => 'Fresh authoritative runtime evidence reports current channels not ready.'],
                'degraded' => ['label' => 'Degraded', 'summary' => 'Current channel evidence is mixed; no whole-world offline state is inferred.'],
                'stale' => ['label' => 'Stale', 'summary' => 'Last accepted runtime evidence is stale and is not treated as current online or offline state.'],
                'unavailable' => ['label' => 'Unavailable', 'summary' => 'No current authoritative runtime evidence is available; no world state is invented.'],
                'invalid' => ['label' => 'Invalid evidence', 'summary' => 'Current runtime evidence failed authority or applicability validation and is not used as world state.'],
            ],
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
