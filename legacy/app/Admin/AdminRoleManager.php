<?php

namespace App\Admin;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use DomainException;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use RuntimeException;

final class AdminRoleManager
{
    public const CONTENT_EDITOR = 'content_editor';

    public const SECURITY_ADMIN = 'security_admin';

    public const PLATFORM_ADMIN = 'platform_admin';

    public function __construct(private readonly AdminAuditRecorder $audit) {}

    /**
     * @return list<string>
     */
    public static function roles(): array
    {
        return [
            self::CONTENT_EDITOR,
            self::SECURITY_ADMIN,
            self::PLATFORM_ADMIN,
        ];
    }

    public function bootstrapFirstPlatformAdmin(int $targetIdentityId): void
    {
        DB::transaction(function () use ($targetIdentityId): void {
            $roleId = $this->roleId(self::PLATFORM_ADMIN, true);

            if (DB::table('identity_admin_roles')->exists()) {
                throw new DomainException('Administrator bootstrap is closed because an administrator role assignment already exists.');
            }

            $target = Identity::query()->lockForUpdate()->find($targetIdentityId);

            if (! $target instanceof Identity) {
                throw new InvalidArgumentException('The target Identity does not exist.');
            }

            if (! $target->hasConfirmedMfa()) {
                throw new DomainException('The first administrator must have confirmed MFA before bootstrap.');
            }

            DB::table('identity_admin_roles')->insert([
                'identity_id' => $target->id,
                'role_id' => $roleId,
            ]);

            $this->audit->record(
                null,
                'admin.bootstrap_first_platform_admin',
                'identity',
                (string) $target->id,
                ['role' => self::PLATFORM_ADMIN],
            );
        }, 3);
    }

    public function assign(Identity $actor, int $targetIdentityId, string $roleKey): bool
    {
        $this->assertKnownRole($roleKey);

        return DB::transaction(function () use ($actor, $targetIdentityId, $roleKey): bool {
            $roleId = $this->roleId($roleKey, true);

            if (! Identity::query()->whereKey($targetIdentityId)->lockForUpdate()->exists()) {
                throw new InvalidArgumentException('The target Identity does not exist.');
            }

            $inserted = DB::table('identity_admin_roles')->insertOrIgnore([
                'identity_id' => $targetIdentityId,
                'role_id' => $roleId,
            ]) === 1;

            if ($inserted) {
                $this->audit->record(
                    $actor->id,
                    'admin.role_assigned',
                    'identity',
                    (string) $targetIdentityId,
                    ['role' => $roleKey],
                );
            }

            return $inserted;
        }, 3);
    }

    public function remove(Identity $actor, int $targetIdentityId, string $roleKey): bool
    {
        $this->assertKnownRole($roleKey);

        return DB::transaction(function () use ($actor, $targetIdentityId, $roleKey): bool {
            $roleId = $this->roleId($roleKey, true);
            $assignments = DB::table('identity_admin_roles')
                ->where('role_id', $roleId)
                ->lockForUpdate()
                ->pluck('identity_id');

            if (
                $roleKey === self::PLATFORM_ADMIN
                && $assignments->contains($targetIdentityId)
                && $assignments->count() <= 1
            ) {
                throw new DomainException('The final platform_admin assignment cannot be removed.');
            }

            $deleted = DB::table('identity_admin_roles')
                ->where('identity_id', $targetIdentityId)
                ->where('role_id', $roleId)
                ->delete() === 1;

            if ($deleted) {
                $this->audit->record(
                    $actor->id,
                    'admin.role_removed',
                    'identity',
                    (string) $targetIdentityId,
                    ['role' => $roleKey],
                );
            }

            return $deleted;
        }, 3);
    }

    private function assertKnownRole(string $roleKey): void
    {
        if (! in_array($roleKey, self::roles(), true)) {
            throw new InvalidArgumentException('Unknown administrator role.');
        }
    }

    private function roleId(string $roleKey, bool $lockForUpdate): int
    {
        $query = DB::table('admin_roles')->where('key', $roleKey);

        if ($lockForUpdate) {
            $query->lockForUpdate();
        }

        $roleId = $query->value('id');

        if (is_int($roleId)) {
            return $roleId;
        }

        if (is_string($roleId) && ctype_digit($roleId)) {
            return (int) $roleId;
        }

        throw new RuntimeException('Configured administrator role is missing or invalid.');
    }
}
