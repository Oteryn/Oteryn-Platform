<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('identities', function (Blueprint $table): void {
            $table->string('account_id', 36)->nullable()->unique()->after('id');
            $table->unsignedBigInteger('native_security_generation')->default(1)->after('game_auth_generation');
        });

        DB::table('identities')->whereNull('account_id')->orderBy('id')->chunkById(100, function ($identities): void {
            foreach ($identities as $identity) {
                do {
                    $accountId = $this->uuidV7();
                } while (DB::table('identities')->where('account_id', $accountId)->exists());

                DB::table('identities')->where('id', $identity->id)->update(['account_id' => $accountId]);
            }
        });

        Schema::table('identities', function (Blueprint $table): void {
            $table->string('account_id', 36)->nullable(false)->change();
        });

        Schema::create('native_game_evidence_observations', function (Blueprint $table): void {
            $table->id();
            $table->char('namespace_hash', 64);
            $table->unsignedBigInteger('source_revision');
            $table->unsignedTinyInteger('version');
            $table->string('operation', 64);
            $table->unsignedBigInteger('source_observed_at');
            $table->text('response_json');
            $table->timestamp('created_at');
            $table->unique(['namespace_hash', 'source_revision'], 'native_evidence_namespace_revision_unique');
            $table->index(['operation', 'created_at'], 'native_evidence_operation_created_index');
        });

        Schema::create('native_game_signing_trust_profiles', function (Blueprint $table): void {
            $table->id();
            $table->string('issuer', 128);
            $table->string('profile', 128);
            $table->string('key_purpose', 256);
            $table->unsignedBigInteger('issuer_revision');
            $table->timestamp('revoked_at')->nullable();
            $table->timestamps();
            $table->unique(['issuer', 'profile', 'key_purpose'], 'native_signing_trust_profile_unique');
        });

        Schema::create('native_game_signing_trust_key_versions', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('profile_id')->constrained('native_game_signing_trust_profiles')->restrictOnDelete();
            $table->string('key_id', 64);
            $table->unsignedBigInteger('key_revision');
            $table->char('public_key', 43);
            $table->boolean('trusted');
            $table->timestamp('revoked_at')->nullable();
            $table->timestamp('created_at');
            $table->unique(['profile_id', 'key_id', 'key_revision'], 'native_signing_key_revision_unique');
            $table->index(['profile_id', 'key_id', 'key_revision'], 'native_signing_key_current_index');
        });
    }

    public function down(): void
    {
        throw new \LogicException(
            'Native AccountId/evidence state is intentionally forward-only: destructive rollback could remint canonical IDs or erase anti-rollback history.',
        );
    }

    private function uuidV7(): string
    {
        $milliseconds = (int) floor(microtime(true) * 1000);
        $bytes = hex2bin(str_pad(dechex($milliseconds), 12, '0', STR_PAD_LEFT)).random_bytes(10);
        if (! is_string($bytes) || strlen($bytes) !== 16) {
            throw new \LogicException('Unable to backfill canonical AccountId.');
        }
        $bytes[6] = chr((ord($bytes[6]) & 0x0f) | 0x70);
        $bytes[8] = chr((ord($bytes[8]) & 0x3f) | 0x80);
        $hex = bin2hex($bytes);

        return substr($hex, 0, 8).'-'.substr($hex, 8, 4).'-'.substr($hex, 12, 4).'-'.substr($hex, 16, 4).'-'.substr($hex, 20, 12);
    }
};
