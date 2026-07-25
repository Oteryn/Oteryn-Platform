<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    private const PERMISSION = 'media.manage';

    public function up(): void
    {
        $now = now();

        DB::table('admin_permissions')->insert([
            'key' => self::PERMISSION,
            'name' => 'Manage the editorial image library',
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    public function down(): void
    {
        DB::table('admin_permissions')
            ->where('key', self::PERMISSION)
            ->delete();
    }
};
