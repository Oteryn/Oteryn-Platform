<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('admin_roles', function (Blueprint $table): void {
            $table->id();
            $table->string('key', 64)->unique();
            $table->string('name', 120);
            $table->timestamps();
        });

        Schema::create('admin_permissions', function (Blueprint $table): void {
            $table->id();
            $table->string('key', 96)->unique();
            $table->string('name', 160);
            $table->timestamps();
        });

        Schema::create('admin_role_permissions', function (Blueprint $table): void {
            $table->foreignId('role_id')->constrained('admin_roles')->cascadeOnDelete();
            $table->foreignId('permission_id')->constrained('admin_permissions')->cascadeOnDelete();
            $table->primary(['role_id', 'permission_id']);
        });

        Schema::create('identity_admin_roles', function (Blueprint $table): void {
            $table->foreignId('identity_id')->constrained('identities')->cascadeOnDelete();
            $table->foreignId('role_id')->constrained('admin_roles')->cascadeOnDelete();
            $table->primary(['identity_id', 'role_id']);
        });

        $now = now();

        DB::table('admin_permissions')->insert([
            ['key' => 'admin.access', 'name' => 'Access administrator surfaces', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'admin.roles.manage', 'name' => 'Manage administrator role assignments', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'audit.view', 'name' => 'View administrator audit events', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'cms.news.manage', 'name' => 'Manage news posts', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'cms.pages.manage', 'name' => 'Manage managed pages', 'created_at' => $now, 'updated_at' => $now],
        ]);

        DB::table('admin_roles')->insert([
            ['key' => 'content_editor', 'name' => 'Content editor', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'security_admin', 'name' => 'Security administrator', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'platform_admin', 'name' => 'Platform administrator', 'created_at' => $now, 'updated_at' => $now],
        ]);

        $permissionIds = DB::table('admin_permissions')->pluck('id', 'key');
        $roleIds = DB::table('admin_roles')->pluck('id', 'key');

        $rolePermissions = [
            'content_editor' => ['admin.access', 'cms.news.manage', 'cms.pages.manage'],
            'security_admin' => ['admin.access', 'admin.roles.manage', 'audit.view'],
            'platform_admin' => ['admin.access', 'admin.roles.manage', 'audit.view', 'cms.news.manage', 'cms.pages.manage'],
        ];

        foreach ($rolePermissions as $roleKey => $permissions) {
            foreach ($permissions as $permissionKey) {
                DB::table('admin_role_permissions')->insert([
                    'role_id' => $roleIds[$roleKey],
                    'permission_id' => $permissionIds[$permissionKey],
                ]);
            }
        }
    }

    public function down(): void
    {
        Schema::dropIfExists('identity_admin_roles');
        Schema::dropIfExists('admin_role_permissions');
        Schema::dropIfExists('admin_permissions');
        Schema::dropIfExists('admin_roles');
    }
};
