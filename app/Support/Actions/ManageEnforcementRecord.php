<?php

namespace App\Support\Actions;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Support\Models\EnforcementRecord;
use App\Support\Notifications\SupportNotificationDeliveryService;
use Carbon\CarbonImmutable;
use DateTimeInterface;
use DomainException;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use InvalidArgumentException;

final class ManageEnforcementRecord
{
    public function __construct(
        private readonly AdminAuditRecorder $audit,
        private readonly SupportNotificationDeliveryService $notifications,
    ) {}

    public function create(
        Identity $actor,
        int $targetIdentityId,
        string $category,
        string $status,
        string $publicReason,
        ?string $moderatorNotes,
        DateTimeInterface $effectiveAt,
        ?DateTimeInterface $expiresAt,
        string $locale = 'en',
    ): EnforcementRecord {
        $this->assertAdminFields($category, $status, $publicReason, $effectiveAt, $expiresAt);

        $record = DB::transaction(function () use (
            $actor,
            $targetIdentityId,
            $category,
            $status,
            $publicReason,
            $moderatorNotes,
            $effectiveAt,
            $expiresAt,
        ): EnforcementRecord {
            $target = Identity::query()->lockForUpdate()->find($targetIdentityId);
            if (! $target instanceof Identity || $target->isTerminated()) {
                throw new InvalidArgumentException('The target Identity is unavailable.');
            }

            $record = EnforcementRecord::query()->create([
                'public_id' => (string) Str::ulid(),
                'identity_id' => $target->id,
                'category' => $category,
                'status' => $status,
                'public_reason' => trim($publicReason),
                'moderator_notes' => $this->nullableText($moderatorNotes),
                'effective_at' => CarbonImmutable::instance($effectiveAt)->utc(),
                'expires_at' => $expiresAt === null ? null : CarbonImmutable::instance($expiresAt)->utc(),
                'appeal_status' => EnforcementRecord::APPEAL_NONE,
                'created_by' => $actor->id,
                'updated_by' => $actor->id,
                'lock_version' => 1,
            ]);

            $this->audit->record(
                $actor->id,
                'support.enforcement_created',
                'enforcement_record',
                $record->public_id,
                [
                    'target_identity_id' => $target->id,
                    'category' => $category,
                    'status' => $status,
                    'has_expiry' => $expiresAt !== null,
                    'has_private_notes' => $record->moderator_notes !== null,
                    'lock_version' => 1,
                ],
            );

            return $record;
        }, 3);

        $target = Identity::query()->find($record->identity_id);
        if ($target instanceof Identity) {
            $this->notifications->queue(
                $target,
                SupportNotificationDeliveryService::ENFORCEMENT_CREATED,
                'enforcement',
                $record->public_id,
                $locale,
            );
        }

        return $record;
    }

    public function update(
        Identity $actor,
        EnforcementRecord $record,
        string $category,
        string $status,
        string $publicReason,
        ?string $moderatorNotes,
        DateTimeInterface $effectiveAt,
        ?DateTimeInterface $expiresAt,
        int $expectedLockVersion,
        string $locale = 'en',
    ): EnforcementRecord {
        $this->assertAdminFields($category, $status, $publicReason, $effectiveAt, $expiresAt);

        $saved = DB::transaction(function () use (
            $actor,
            $record,
            $category,
            $status,
            $publicReason,
            $moderatorNotes,
            $effectiveAt,
            $expiresAt,
            $expectedLockVersion,
        ): EnforcementRecord {
            $current = EnforcementRecord::query()->lockForUpdate()->findOrFail($record->id);
            $this->assertVersion($current, $expectedLockVersion);

            $current->forceFill([
                'category' => $category,
                'status' => $status,
                'public_reason' => trim($publicReason),
                'moderator_notes' => $this->nullableText($moderatorNotes),
                'effective_at' => CarbonImmutable::instance($effectiveAt)->utc(),
                'expires_at' => $expiresAt === null ? null : CarbonImmutable::instance($expiresAt)->utc(),
                'updated_by' => $actor->id,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            $this->audit->record(
                $actor->id,
                'support.enforcement_updated',
                'enforcement_record',
                $current->public_id,
                [
                    'target_identity_id' => $current->identity_id,
                    'category' => $current->category,
                    'status' => $current->status,
                    'has_expiry' => $current->expires_at !== null,
                    'has_private_notes' => $current->moderator_notes !== null,
                    'lock_version' => $current->lock_version,
                ],
            );

            return $current;
        }, 3);

        $target = Identity::query()->find($saved->identity_id);
        if ($target instanceof Identity) {
            $this->notifications->queue(
                $target,
                SupportNotificationDeliveryService::ENFORCEMENT_UPDATED,
                'enforcement',
                $saved->public_id,
                $locale,
            );
        }

        return $saved;
    }

    public function acknowledge(Identity $identity, EnforcementRecord $record, int $expectedLockVersion): EnforcementRecord
    {
        return DB::transaction(function () use ($identity, $record, $expectedLockVersion): EnforcementRecord {
            $current = EnforcementRecord::query()->lockForUpdate()->findOrFail($record->id);
            $this->assertOwner($identity, $current);
            $this->assertVersion($current, $expectedLockVersion);

            if ($current->acknowledged_at !== null) {
                return $current;
            }

            $current->forceFill([
                'acknowledged_at' => now(),
                'lock_version' => $current->lock_version + 1,
            ])->save();

            return $current;
        }, 3);
    }

    public function appeal(
        Identity $identity,
        EnforcementRecord $record,
        string $message,
        int $expectedLockVersion,
    ): EnforcementRecord {
        $normalized = trim($message);
        if ($normalized === '') {
            throw new InvalidArgumentException('An appeal message is required.');
        }

        return DB::transaction(function () use ($identity, $record, $normalized, $expectedLockVersion): EnforcementRecord {
            $current = EnforcementRecord::query()->lockForUpdate()->findOrFail($record->id);
            $this->assertOwner($identity, $current);
            $this->assertVersion($current, $expectedLockVersion);

            if (! in_array($current->appeal_status, [EnforcementRecord::APPEAL_NONE, EnforcementRecord::APPEAL_REJECTED], true)) {
                throw new DomainException('This enforcement record already has an active or accepted appeal.');
            }

            $current->forceFill([
                'appeal_status' => EnforcementRecord::APPEAL_REQUESTED,
                'appeal_message' => $normalized,
                'appeal_outcome' => null,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            return $current;
        }, 3);
    }

    public function resolveAppeal(
        Identity $actor,
        EnforcementRecord $record,
        string $appealStatus,
        string $outcome,
        int $expectedLockVersion,
        string $locale = 'en',
    ): EnforcementRecord {
        if (! in_array($appealStatus, [
            EnforcementRecord::APPEAL_REVIEWING,
            EnforcementRecord::APPEAL_ACCEPTED,
            EnforcementRecord::APPEAL_REJECTED,
        ], true)) {
            throw new InvalidArgumentException('Unsupported appeal status.');
        }

        $normalizedOutcome = trim($outcome);
        if (in_array($appealStatus, [EnforcementRecord::APPEAL_ACCEPTED, EnforcementRecord::APPEAL_REJECTED], true) && $normalizedOutcome === '') {
            throw new InvalidArgumentException('A public-safe appeal outcome is required.');
        }

        $saved = DB::transaction(function () use (
            $actor,
            $record,
            $appealStatus,
            $normalizedOutcome,
            $expectedLockVersion,
        ): EnforcementRecord {
            $current = EnforcementRecord::query()->lockForUpdate()->findOrFail($record->id);
            $this->assertVersion($current, $expectedLockVersion);

            if (! in_array($current->appeal_status, [EnforcementRecord::APPEAL_REQUESTED, EnforcementRecord::APPEAL_REVIEWING], true)) {
                throw new DomainException('No active appeal can be reviewed for this enforcement record.');
            }

            $current->forceFill([
                'appeal_status' => $appealStatus,
                'appeal_outcome' => $normalizedOutcome === '' ? null : $normalizedOutcome,
                'updated_by' => $actor->id,
                'status' => $appealStatus === EnforcementRecord::APPEAL_ACCEPTED
                    ? EnforcementRecord::STATUS_REVOKED
                    : $current->status,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            $this->audit->record(
                $actor->id,
                'support.enforcement_appeal_updated',
                'enforcement_record',
                $current->public_id,
                [
                    'target_identity_id' => $current->identity_id,
                    'appeal_status' => $current->appeal_status,
                    'status' => $current->status,
                    'has_outcome' => $current->appeal_outcome !== null,
                    'lock_version' => $current->lock_version,
                ],
            );

            return $current;
        }, 3);

        if (in_array($saved->appeal_status, [EnforcementRecord::APPEAL_ACCEPTED, EnforcementRecord::APPEAL_REJECTED], true)) {
            $target = Identity::query()->find($saved->identity_id);
            if ($target instanceof Identity) {
                $this->notifications->queue(
                    $target,
                    SupportNotificationDeliveryService::APPEAL_OUTCOME,
                    'enforcement',
                    $saved->public_id,
                    $locale,
                );
            }
        }

        return $saved;
    }

    private function assertAdminFields(
        string $category,
        string $status,
        string $publicReason,
        DateTimeInterface $effectiveAt,
        ?DateTimeInterface $expiresAt,
    ): void {
        if (! in_array($category, EnforcementRecord::categories(), true)) {
            throw new InvalidArgumentException('Unsupported enforcement category.');
        }
        if (! in_array($status, EnforcementRecord::statuses(), true)) {
            throw new InvalidArgumentException('Unsupported enforcement status.');
        }
        if (trim($publicReason) === '') {
            throw new InvalidArgumentException('A public-safe reason is required.');
        }
        if ($expiresAt !== null && ! CarbonImmutable::instance($expiresAt)->isAfter(CarbonImmutable::instance($effectiveAt))) {
            throw new InvalidArgumentException('An enforcement expiry must be after its effective time.');
        }
    }

    private function assertOwner(Identity $identity, EnforcementRecord $record): void
    {
        if ($record->identity_id !== $identity->id) {
            abort(404);
        }
    }

    private function assertVersion(EnforcementRecord $record, int $expectedLockVersion): void
    {
        if ($record->lock_version !== $expectedLockVersion) {
            throw new DomainException('This enforcement record changed after the page was opened. Reload it before continuing.');
        }
    }

    private function nullableText(?string $value): ?string
    {
        $normalized = trim((string) $value);

        return $normalized === '' ? null : $normalized;
    }
}
