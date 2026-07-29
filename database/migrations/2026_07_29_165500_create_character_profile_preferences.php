<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('character_profile_preferences', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('identity_id')->constrained('identities')->cascadeOnDelete();
            $table->unsignedBigInteger('canary_player_id');
            $table->text('public_comment')->nullable();
            $table->boolean('show_account_association')->default(false);
            $table->boolean('show_status')->default(false);
            $table->boolean('show_guild')->default(true);
            $table->boolean('show_house')->default(true);
            $table->boolean('show_skills')->default(true);
            $table->boolean('show_deaths')->default(true);
            $table->boolean('show_kills')->default(true);
            $table->boolean('is_main_character')->default(false);
            $table->timestamps();

            $table->unique(['identity_id', 'canary_player_id'], 'character_profile_preferences_owner_player_unique');
            $table->index(['identity_id', 'is_main_character'], 'character_profile_preferences_main_index');
            $table->index('canary_player_id', 'character_profile_preferences_player_index');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('character_profile_preferences');
    }
};
