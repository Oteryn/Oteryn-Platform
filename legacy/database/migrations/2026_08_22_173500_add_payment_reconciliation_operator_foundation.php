<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    private const PERMISSION = 'payments.reconcile';

    public function up(): void
    {
        Schema::create('payment_reconciliation_resolutions', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('payment_reconciliation_entry_id');
            $table->foreign('payment_reconciliation_entry_id', 'payment_recon_resolution_entry_fk')
                ->references('id')
                ->on('payment_reconciliation_entries')
                ->restrictOnDelete();
            $table->foreignId('actor_identity_id')
                ->constrained('identities')
                ->restrictOnDelete();
            $table->string('resolution_code', 64);
            $table->string('idempotency_key', 120)->unique();
            $table->timestamp('created_at');

            $table->index(
                ['payment_reconciliation_entry_id', 'created_at'],
                'payment_reconciliation_resolution_history',
            );
        });

        $now = now();
        DB::table('admin_permissions')->insertOrIgnore([
            'key' => self::PERMISSION,
            'name' => 'Review deterministic payment reconciliation evidence',
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        $permissionId = DB::table('admin_permissions')
            ->where('key', self::PERMISSION)
            ->value('id');

        if (! is_int($permissionId) && ! (is_string($permissionId) && ctype_digit($permissionId))) {
            throw new RuntimeException('The payments.reconcile permission could not be persisted.');
        }

        $platformAdminRoleId = DB::table('admin_roles')
            ->where('key', 'platform_admin')
            ->value('id');

        if (is_int($platformAdminRoleId)
            || (is_string($platformAdminRoleId) && ctype_digit($platformAdminRoleId))) {
            DB::table('admin_role_permissions')->insertOrIgnore([
                'role_id' => (int) $platformAdminRoleId,
                'permission_id' => (int) $permissionId,
            ]);
        }
    }

    public function down(): void
    {
        if (DB::table('payment_reconciliation_resolutions')->exists()) {
            throw new RuntimeException(
                'Payment reconciliation evidence exists and cannot be removed by migration rollback.',
            );
        }

        $permissionId = DB::table('admin_permissions')
            ->where('key', self::PERMISSION)
            ->value('id');

        if (is_int($permissionId) || (is_string($permissionId) && ctype_digit($permissionId))) {
            DB::table('admin_role_permissions')
                ->where('permission_id', (int) $permissionId)
                ->delete();
        }

        DB::table('admin_permissions')
            ->where('key', self::PERMISSION)
            ->delete();

        Schema::dropIfExists('payment_reconciliation_resolutions');
    }
};
