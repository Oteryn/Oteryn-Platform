<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    /**
     * @var list<string>
     */
    private const KEYS = [
        'portal.access',
        'portal.announcements.manage',
        'portal.settings.manage',
        'downloads.manage',
        'events.manage',
        'events.publish',
        'support.content.manage',
        'wiki.access',
        'wiki.articles.manage',
        'wiki.categories.manage',
        'wiki.publish',
    ];

    public function up(): void
    {
        $now = now();

        DB::table('admin_permissions')->insert([
            ['key' => 'portal.access', 'name' => 'Access public portal administration', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'portal.announcements.manage', 'name' => 'Manage public portal announcements', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'portal.settings.manage', 'name' => 'Manage approved public portal settings', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'downloads.manage', 'name' => 'Manage client download records', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'events.manage', 'name' => 'Manage event records', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'events.publish', 'name' => 'Publish events', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'support.content.manage', 'name' => 'Manage support content', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'wiki.access', 'name' => 'Access Wiki administration', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'wiki.articles.manage', 'name' => 'Manage Wiki articles', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'wiki.categories.manage', 'name' => 'Manage Wiki categories', 'created_at' => $now, 'updated_at' => $now],
            ['key' => 'wiki.publish', 'name' => 'Publish Wiki content', 'created_at' => $now, 'updated_at' => $now],
        ]);
    }

    public function down(): void
    {
        DB::table('admin_permissions')
            ->whereIn('key', self::KEYS)
            ->delete();
    }
};
