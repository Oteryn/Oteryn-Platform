<?php

namespace App\Identity\Actions;

use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use App\Identity\Sessions\IdentityWebSessionRegistry;
use Illuminate\Support\Facades\DB;

final class RevokeIdentityWebSessions
{
    public function __construct(
        private readonly SecurityEventRecorder $securityEvents,
        private readonly IdentityWebSessionRegistry $registry,
    ) {}

    public function execute(Identity $identity): int
    {
        return DB::transaction(function () use ($identity): int {
            Identity::query()
                ->whereKey($identity->id)
                ->increment('web_session_generation');

            $identity->refresh();
            $this->registry->revokeAll($identity);
            $this->securityEvents->recordIdentityWebSessionsRevoked($identity->id);

            return $identity->web_session_generation;
        });
    }
}
