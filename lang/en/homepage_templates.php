<?php

return [
    'admin' => [
        'title' => 'Homepage templates',
        'eyebrow' => 'Portal presentation',
        'heading' => 'Homepage template',
        'intro' => 'Preview and activate only code-approved homepage presentations. Public requests cannot choose a template.',
        'current' => 'Current template',
        'version' => 'Version :version',
        'active' => 'Active',
        'preview' => 'Preview',
        'activate' => 'Activate',
        'rollback' => 'Roll back to :template',
        'rollback_help' => 'No previous approved template is available for rollback.',
        'drift_warning' => 'The stored homepage template is no longer registered. Public traffic is safely using the production default until an approved template is selected.',
        'activated' => 'Homepage template activated.',
        'rolled_back' => 'Homepage template rolled back.',
        'conflict' => 'The homepage template changed in another session. Reload this page and try again.',
        'rollback_unavailable' => 'There is no registered previous homepage template to restore.',
        'dashboard_title' => 'Homepage templates',
        'dashboard_help' => 'Preview, activate and roll back approved homepage presentations.',
    ],
    'templates' => [
        'production' => [
            'label' => 'Production',
            'description' => 'The current Oteryn dark-fantasy homepage and the deterministic public default.',
        ],
        'classic' => [
            'label' => 'Classic portal',
            'description' => 'The earlier reviewed portal presentation, restored only behind the administrator selector.',
        ],
    ],
];
