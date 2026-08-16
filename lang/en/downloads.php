<?php

return [
    'updater' => [
        'title' => 'Automatic updater state',
        'trust_notice' => 'This page reports Platform reconciliation of public TUF metadata only. The first-party updater independently verifies TUF signatures, freshness and trusted target metadata; the Download Center is not a trust root.',
        'policy' => 'Policy revision :revision · :mode update · minimum supported sequence :minimum · metadata expires :expires UTC.',
        'states' => [
            'browser_only' => 'Browser download only. This release has no automatic-updater identity.',
            'pending' => 'Updater identity exists, but no signed generation selecting this browser release is Platform-active.',
            'active' => 'Platform-active signed generation selects this exact release. Client-side TUF verification is still required.',
            'browser_mismatch' => 'Browser current and updater current do not match. The browser download remains available, but this page does not silently treat it as updater-current.',
            'withdrawn' => 'This updater release is withdrawn from new policy selection. Browser publication remains a separate state.',
            'revoked' => 'The Platform-active signed policy revokes this release identity for automatic updating.',
            'target_revoked' => 'The Platform-active signed policy revokes at least one exact platform/architecture target in this current release.',
            'trust_expired' => 'The last Platform-active updater metadata has expired. Automatic-update authority is unavailable until fresh trusted state is reconciled.',
            'degraded' => 'Updater state is ambiguous or unavailable and is shown fail-closed. Browser publication remains separate.',
        ],
    ],
];