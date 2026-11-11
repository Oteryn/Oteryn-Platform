<?php

namespace App\Audit;

use Illuminate\Support\Facades\DB;

final class AdminAuditRecorder
{
    /**
     * @param  array<string, bool|int|string|null>  $metadata
     */
    public function record(
        ?int $actorIdentityId,
        string $action,
        string $targetType,
        ?string $targetId = null,
        array $metadata = [],
    ): void {
        DB::table('admin_audit_events')->insert([
            'actor_identity_id' => $actorIdentityId,
            'action' => $action,
            'target_type' => $targetType,
            'target_id' => $targetId,
            'metadata' => $metadata === [] ? null : json_encode($metadata, JSON_THROW_ON_ERROR),
            'occurred_at' => now(),
        ]);
    }
}
