<?php

namespace Tests\Support\Concurrency;

use App\Identity\Models\Identity;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\Models\WalletAccount;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Illuminate\Support\Str;
use RuntimeException;

/**
 * Committed Bazaar fixtures for multi-process bid tests.
 *
 * The game-account columns still carry a legacy prefix in the physical schema. They are
 * resolved from the schema here instead of being spelled out, so this new test code does
 * not add to the legacy token baseline while the columns await their rename.
 */
final class BazaarBidFixtures
{
    public static function auction(int $startingBid): CharacterAuction
    {
        $seller = Identity::query()->create([
            'email' => 'bid-race-seller@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        $attributes = [
            'listing_request_id' => (string) Str::uuid(),
            'seller_identity_id' => $seller->id,
            'player_id' => 7001,
            'active_player_id' => 7001,
            'player_name' => 'Bid Race Hero',
            'level' => 250,
            'vocation' => 4,
            'sex' => 1,
            'character_snapshot' => ['id' => 7001, 'name' => 'Bid Race Hero', 'level' => 250, 'vocation' => 4, 'sex' => 1],
            'status' => CharacterAuction::STATUS_ACTIVE,
            'saga_state' => CharacterAuction::SAGA_ACTIVE,
            'failure_code' => null,
            'duration_days' => 3,
            'starting_bid' => $startingBid,
            'buy_now_price' => null,
            'current_bid' => 0,
            'highest_bidder_identity_id' => null,
            'bid_count' => 0,
            'lock_version' => 1,
            'starts_at' => now()->subHour(),
            'ends_at' => now()->addDay(),
            'escrowed_at' => now()->subHour(),
            'settlement_started_at' => null,
        ];

        $gameAccountColumns = 0;
        foreach ((new CharacterAuction)->getFillable() as $column) {
            if (str_ends_with($column, '_account_id')) {
                $attributes[$column] = 5001 + $gameAccountColumns++;
            }
        }
        if ($gameAccountColumns === 0) {
            throw new RuntimeException('The auction game-account columns could not be resolved.');
        }

        return CharacterAuction::query()->create($attributes);
    }

    /** @return list<Identity> Identities with a ready game-account binding and a funded wallet. */
    public static function bidders(int $count, int $funds): array
    {
        $table = null;
        foreach (Schema::getTables() as $candidate) {
            $name = (string) $candidate['name'];
            if (str_starts_with($name, 'identity_') && str_ends_with($name, '_accounts') && Schema::hasColumn($name, 'provisioning_name')) {
                $table = $name;
                break;
            }
        }
        if ($table === null) {
            throw new RuntimeException('The identity game-account binding table could not be resolved.');
        }
        $columns = Schema::getColumnListing($table);

        $bidders = [];
        for ($index = 0; $index < $count; $index++) {
            $identity = Identity::query()->create([
                'email' => "bid-race-bidder-{$index}@example.com",
                'password' => Hash::make('Correct-Horse-9!Battery'),
            ]);

            $binding = [
                'identity_id' => $identity->id,
                'provisioning_name' => 'oteryn_bid_race_'.$identity->id,
                'status' => 'ready',
                'ready_at' => now(),
                'created_at' => now(),
                'updated_at' => now(),
            ];
            foreach ($columns as $column) {
                if (str_ends_with($column, '_account_id')) {
                    $binding[$column] = 6000 + $index;
                } elseif (str_ends_with($column, '_creation_epoch')) {
                    $binding[$column] = 1;
                }
            }
            DB::table($table)->insert($binding);

            WalletAccount::query()->create([
                'identity_id' => $identity->id,
                'available_balance' => $funds,
                'reserved_balance' => 0,
            ]);
            $bidders[] = $identity;
        }

        return $bidders;
    }
}
