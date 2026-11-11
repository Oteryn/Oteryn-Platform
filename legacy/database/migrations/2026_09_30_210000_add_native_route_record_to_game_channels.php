<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Native Registry route record per issued Channel (OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT §7.3,
 * Decision D3; D172 testing/preproduction only). Additive, nullable: a Channel without a complete
 * record, or with native login disabled, is never a route candidate (§7.4 conditions 1 and 2).
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::table('game_channels', function (Blueprint $table): void {
            $table->string('native_route_host', 253)->nullable();
            $table->unsignedSmallInteger('native_route_port')->nullable();
            $table->string('native_route_tls_server_name', 253)->nullable();
            $table->unsignedInteger('native_route_version')->nullable();
            $table->string('native_route_revision', 64)->nullable();
            $table->boolean('native_login_enabled')->default(false);
        });
    }

    public function down(): void
    {
        // A published route record is bound into outstanding grants: refuse before DDL while any exists.
        if (Schema::hasColumn('game_channels', 'native_route_revision')
            && DB::table('game_channels')->whereNotNull('native_route_revision')->exists()) {
            throw new LogicException('Published native route records must be retained; rollback refused before DDL.');
        }

        $columns = array_values(array_filter(
            ['native_route_host', 'native_route_port', 'native_route_tls_server_name', 'native_route_version', 'native_route_revision', 'native_login_enabled'],
            static fn (string $column): bool => Schema::hasColumn('game_channels', $column),
        ));
        if ($columns !== []) {
            Schema::table('game_channels', function (Blueprint $table) use ($columns): void {
                $table->dropColumn($columns);
            });
        }
    }
};
