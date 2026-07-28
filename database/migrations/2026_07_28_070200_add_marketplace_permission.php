<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    public function up(): void
    {
        $now = now();

        DB::table('admin_permissions')->insert([
            'key' => 'marketplace.manage',
            'name' => 'Manage Character Bazaar wallets and recovery',
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    public function down(): void
    {
        DB::table('admin_permissions')->where('key', 'marketplace.manage')->delete();
    }
};
