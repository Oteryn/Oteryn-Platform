<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use RuntimeException;

return new class extends Migration
{
    /** @var list<string> */
    private const PERMISSIONS = [
        'support.tickets.manage',
        'support.reports.manage',
        'support.enforcement.manage',
    ];

    /** @var list<string> */
    private const ROLE_KEYS = [
        'security_admin',
        'platform_admin',
    ];

    public function up(): void
    {
        Schema::create('support_tickets', function (Blueprint $table): void {
            $table->id();
            $table->char('public_id', 26)->unique();
            $table->foreignId('identity_id')->constrained('identities')->restrictOnDelete();
            $table->string('request_key', 64);
            $table->string('category', 32);
            $table->string('subject', 160);
            $table->string('status', 32)->default('open');
            $table->unsignedInteger('lock_version')->default(1);
            $table->timestamp('last_message_at');
            $table->timestamp('closed_at')->nullable();
            $table->timestamps();

            $table->unique(['identity_id', 'request_key']);
            $table->index(['identity_id', 'status', 'last_message_at']);
            $table->index(['status', 'last_message_at']);
        });

        Schema::create('support_ticket_messages', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('support_ticket_id')->constrained('support_tickets')->cascadeOnDelete();
            $table->foreignId('author_identity_id')->nullable()->constrained('identities')->nullOnDelete();
            $table->string('author_kind', 16);
            $table->string('visibility', 16)->default('public');
            $table->text('body');
            $table->timestamps();

            $table->index(
                ['support_ticket_id', 'visibility', 'created_at'],
                'support_msg_ticket_visibility_created_idx',
            );
        });

        Schema::create('player_reports', function (Blueprint $table): void {
            $table->id();
            $table->char('public_id', 26)->unique();
            $table->foreignId('reporter_identity_id')->constrained('identities')->restrictOnDelete();
            $table->string('request_key', 64);
            $table->string('report_type', 24);
            $table->string('category', 40);
            $table->string('target_reference', 160);
            $table->text('evidence_summary')->nullable();
            $table->string('status', 24)->default('submitted');
            $table->text('public_outcome')->nullable();
            $table->text('moderator_notes')->nullable();
            $table->foreignId('assigned_to')->nullable()->constrained('identities')->nullOnDelete();
            $table->unsignedInteger('lock_version')->default(1);
            $table->timestamp('processed_at')->nullable();
            $table->timestamps();

            $table->unique(['reporter_identity_id', 'request_key']);
            $table->index(['reporter_identity_id', 'status', 'created_at']);
            $table->index(['status', 'created_at']);
        });

        Schema::create('enforcement_records', function (Blueprint $table): void {
            $table->id();
            $table->char('public_id', 26)->unique();
            $table->foreignId('identity_id')->constrained('identities')->restrictOnDelete();
            $table->string('category', 32);
            $table->string('status', 24)->default('active');
            $table->text('public_reason');
            $table->text('moderator_notes')->nullable();
            $table->timestamp('effective_at');
            $table->timestamp('expires_at')->nullable();
            $table->timestamp('acknowledged_at')->nullable();
            $table->string('appeal_status', 24)->default('none');
            $table->text('appeal_message')->nullable();
            $table->text('appeal_outcome')->nullable();
            $table->foreignId('created_by')->nullable()->constrained('identities')->nullOnDelete();
            $table->foreignId('updated_by')->nullable()->constrained('identities')->nullOnDelete();
            $table->unsignedInteger('lock_version')->default(1);
            $table->timestamps();

            $table->index(['identity_id', 'status', 'effective_at']);
            $table->index(['status', 'effective_at']);
            $table->index(['appeal_status', 'updated_at']);
        });

        Schema::create('support_notification_deliveries', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('identity_id')->constrained('identities')->cascadeOnDelete();
            $table->string('event_key', 64);
            $table->string('related_type', 32);
            $table->string('related_id', 64);
            $table->char('locale', 2)->default('en');
            $table->string('status', 16)->default('pending');
            $table->unsignedSmallInteger('attempts')->default(0);
            $table->string('last_error_code', 64)->nullable();
            $table->timestamp('sent_at')->nullable();
            $table->timestamp('failed_at')->nullable();
            $table->timestamps();

            $table->unique(['identity_id', 'event_key', 'related_type', 'related_id'], 'support_notification_unique');
            $table->index(['identity_id', 'status', 'created_at']);
        });

        $now = now();
        foreach (self::PERMISSIONS as $permission) {
            DB::table('admin_permissions')->insertOrIgnore([
                'key' => $permission,
                'name' => match ($permission) {
                    'support.tickets.manage' => 'Manage support tickets',
                    'support.reports.manage' => 'Manage player and content reports',
                    'support.enforcement.manage' => 'Manage Platform enforcement records',
                },
                'created_at' => $now,
                'updated_at' => $now,
            ]);
        }

        foreach (self::ROLE_KEYS as $roleKey) {
            $roleId = $this->requiredId('admin_roles', 'key', $roleKey);

            foreach (self::PERMISSIONS as $permission) {
                DB::table('admin_role_permissions')->insertOrIgnore([
                    'role_id' => $roleId,
                    'permission_id' => $this->requiredId('admin_permissions', 'key', $permission),
                ]);
            }
        }
    }

    public function down(): void
    {
        $permissionIds = [];
        foreach (DB::table('admin_permissions')->whereIn('key', self::PERMISSIONS)->pluck('id') as $permissionId) {
            if (is_int($permissionId)) {
                $permissionIds[] = $permissionId;
            } elseif (is_string($permissionId) && ctype_digit($permissionId)) {
                $permissionIds[] = (int) $permissionId;
            }
        }

        if ($permissionIds !== []) {
            DB::table('admin_role_permissions')->whereIn('permission_id', $permissionIds)->delete();
        }
        DB::table('admin_permissions')->whereIn('key', self::PERMISSIONS)->delete();

        Schema::dropIfExists('support_notification_deliveries');
        Schema::dropIfExists('enforcement_records');
        Schema::dropIfExists('player_reports');
        Schema::dropIfExists('support_ticket_messages');
        Schema::dropIfExists('support_tickets');
    }

    private function requiredId(string $table, string $keyColumn, string $key): int
    {
        $id = DB::table($table)->where($keyColumn, $key)->value('id');

        if (is_int($id)) {
            return $id;
        }

        if (is_string($id) && ctype_digit($id)) {
            return (int) $id;
        }

        throw new RuntimeException("Required RBAC record {$key} is missing.");
    }
};
