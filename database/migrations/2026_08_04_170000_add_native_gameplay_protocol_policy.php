<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('game_worlds', function (Blueprint $table): void {
            $table->unsignedBigInteger('gameplay_policy_revision')->default(1)->after('game_port');
        });

        Schema::create('game_world_protocol_candidates', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('game_world_id')->constrained('game_worlds')->cascadeOnDelete();
            $table->unsignedBigInteger('channel_id')->default(1);
            $table->unsignedSmallInteger('sort_order');
            $table->string('family', 64);
            $table->string('profile', 64);
            $table->string('transport', 64);
            $table->unsignedInteger('schema_revision');
            $table->char('schema_sha256', 64);
            $table->json('required_capabilities');
            $table->json('optional_capabilities');
            $table->string('endpoint_id', 64);
            $table->string('game_host', 255);
            $table->unsignedSmallInteger('game_port');
            $table->string('tls_server_name', 255);
            $table->boolean('enabled')->default(false)->index();
            $table->timestamps();

            $table->unique(
                ['game_world_id', 'channel_id', 'sort_order'],
                'world_protocol_candidate_order_unique',
            );
            $table->unique(
                ['game_world_id', 'channel_id', 'family', 'profile', 'transport', 'schema_revision'],
                'world_protocol_candidate_tuple_unique',
            );
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('game_world_protocol_candidates');

        Schema::table('game_worlds', function (Blueprint $table): void {
            $table->dropColumn('gameplay_policy_revision');
        });
    }
};
