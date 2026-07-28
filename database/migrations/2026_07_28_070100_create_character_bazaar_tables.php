<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('character_auctions', function (Blueprint $table): void {
            $table->id();
            $table->uuid('listing_request_id')->unique();
            $table->foreignId('seller_identity_id')->constrained('identities')->restrictOnDelete();
            $table->unsignedBigInteger('seller_canary_account_id');
            $table->unsignedBigInteger('escrow_canary_account_id');
            $table->unsignedBigInteger('player_id');
            $table->unsignedBigInteger('active_player_id')->nullable()->unique();
            $table->string('player_name', 255)->index();
            $table->unsignedInteger('level')->index();
            $table->unsignedInteger('vocation')->index();
            $table->unsignedInteger('sex');
            $table->json('character_snapshot');
            $table->string('status', 32)->index();
            $table->string('saga_state', 40)->index();
            $table->string('failure_code', 64)->nullable();
            $table->unsignedInteger('duration_days');
            $table->unsignedBigInteger('starting_bid');
            $table->unsignedBigInteger('buy_now_price')->nullable();
            $table->unsignedBigInteger('current_bid')->default(0);
            $table->foreignId('highest_bidder_identity_id')->nullable()->constrained('identities')->nullOnDelete();
            $table->unsignedInteger('bid_count')->default(0);
            $table->unsignedInteger('lock_version')->default(1);
            $table->timestamp('starts_at')->nullable()->index();
            $table->timestamp('ends_at')->nullable()->index();
            $table->timestamp('escrowed_at')->nullable();
            $table->timestamp('settlement_started_at')->nullable();
            $table->timestamp('settled_at')->nullable();
            $table->timestamp('cancelled_at')->nullable();
            $table->timestamps();

            $table->index(['status', 'ends_at'], 'character_auctions_close_lookup');
            $table->index(['seller_identity_id', 'created_at'], 'character_auctions_seller_history');
            $table->index(['highest_bidder_identity_id', 'created_at'], 'character_auctions_bidder_history');
        });

        Schema::create('character_auction_bids', function (Blueprint $table): void {
            $table->id();
            $table->uuid('request_id')->unique();
            $table->foreignId('auction_id')->constrained('character_auctions')->cascadeOnDelete();
            $table->foreignId('bidder_identity_id')->constrained('identities')->restrictOnDelete();
            $table->unsignedBigInteger('amount');
            $table->string('status', 24)->index();
            $table->timestamp('placed_at');
            $table->timestamp('updated_at')->nullable();

            $table->index(['auction_id', 'placed_at'], 'character_auction_bid_history');
            $table->index(['bidder_identity_id', 'placed_at'], 'character_auction_bidder_history');
        });

        Schema::create('character_auction_watches', function (Blueprint $table): void {
            $table->foreignId('identity_id')->constrained('identities')->cascadeOnDelete();
            $table->foreignId('auction_id')->constrained('character_auctions')->cascadeOnDelete();
            $table->timestamp('created_at');
            $table->primary(['identity_id', 'auction_id']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('character_auction_watches');
        Schema::dropIfExists('character_auction_bids');
        Schema::dropIfExists('character_auctions');
    }
};
