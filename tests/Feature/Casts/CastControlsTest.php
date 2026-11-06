<?php

namespace Tests\Feature\Casts;

use App\Casts\CastControls;
use App\Casts\CastUnavailable;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Symfony\Component\Process\Process;
use Tests\TestCase;

final class CastControlsTest extends TestCase
{
    use DatabaseMigrations;

    private const NODE = '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b';

    private const CAST = 'abcdef0123456789abcdef0123456789';

    public function test_password_is_optional_and_wrong_password_is_uniformly_unavailable(): void
    {
        $service = app(CastControls::class);
        $owner = Identity::query()->create(['email' => (string) Str::uuid7().'@example.invalid', 'password' => Hash::make('Test-Password-17!')]);
        $viewer = Identity::query()->create(['email' => (string) Str::uuid7().'@example.invalid', 'password' => Hash::make('Test-Password-17!')]);
        $service->announce(self::NODE, $owner->account_id, self::CAST, 1, 1, 0, 50);
        $public = $service->reserve($viewer, self::CAST, null, '127.0.0.1');
        self::assertNotEmpty($public->reservationId);
        $service->release(self::NODE, $public->reservationId);
        $service->password($owner, self::CAST, 1, 'correct cast password');
        try {
            $service->reserve($viewer, self::CAST, 'wrong', '127.0.0.1');
            self::fail('Wrong cast password was admitted.');
        } catch (CastUnavailable $failure) {
            self::assertSame('Cast unavailable', $failure->getMessage());
        }
        self::assertSame(0, DB::table('cast_viewer_reservations')->count());
        $private = $service->reserve($viewer, self::CAST, 'correct cast password', '127.0.0.1');
        self::assertSame(2, $private->accessRevision);
    }

    public function test_logged_in_accounts_need_no_character_and_have_two_global_slots(): void
    {
        $service = app(CastControls::class);
        $owner = Identity::query()->create(['email' => (string) Str::uuid7().'@example.invalid', 'password' => Hash::make('Test-Password-17!')]);
        $viewer = Identity::query()->create(['email' => (string) Str::uuid7().'@example.invalid', 'password' => Hash::make('Test-Password-17!')]);
        $service->announce(self::NODE, $owner->account_id, self::CAST, 1, 1, 0, 50);
        $second = str_repeat('1', 32);
        $service->announce(self::NODE, $owner->account_id, $second, 1, 1, 0, 50);
        $first = $service->reserve($viewer, self::CAST, null, '127.0.0.1');
        $service->reserve($viewer, $second, null, '127.0.0.1');
        try {
            $service->reserve($viewer, self::CAST, null, '127.0.0.1');
            self::fail('Third global account slot was admitted.');
        } catch (CastUnavailable) {
            self::assertSame(2, DB::table('cast_viewer_reservations')->count());
        }
        $service->release(self::NODE, $first->reservationId);
        $service->reserve($viewer, self::CAST, null, '127.0.0.1');
        $this->expectException(CastUnavailable::class);
        $service->reserve(null, self::CAST, null, '127.0.0.1');
    }

    public function test_observer_ticket_is_single_use_node_bound_and_kick_releases_global_slot(): void
    {
        $service = app(CastControls::class);
        $owner = Identity::query()->create(['email' => 'owner@example.invalid', 'password' => 'unused']);
        $viewer = Identity::query()->create(['email' => 'viewer@example.invalid', 'password' => 'unused']);
        $service->announce(self::NODE, $owner->account_id, self::CAST, 1, 1, 0, 1);
        $grant = $service->reserve($viewer, self::CAST, null, '127.0.0.1');
        self::assertNotEmpty($grant->ticket);
        $service->consumeTicket(self::NODE, $grant->ticket);
        try {
            $service->consumeTicket(self::NODE, $grant->ticket);
            self::fail('Replayed observer ticket was admitted.');
        } catch (CastUnavailable) {
            self::assertSame(1, DB::table('cast_viewer_reservations')->count());
        }
        try {
            $service->reserve($viewer, self::CAST, null, '127.0.0.1');
            self::fail('Cast viewer limit was exceeded.');
        } catch (CastUnavailable) {
            self::assertSame(1, DB::table('cast_viewer_reservations')->count());
        }
        $service->release('0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a70', $grant->reservationId);
        self::assertSame(1, DB::table('cast_viewer_reservations')->count());
        $service->release(self::NODE, $grant->reservationId);
        self::assertSame(0, DB::table('cast_viewer_reservations')->count());
        $service->reserve($viewer, self::CAST, null, '127.0.0.1');
    }

    public function test_fifty_viewer_ceiling_and_expiry_are_enforced_in_postgres(): void
    {
        $this->freezeTime();
        $service = app(CastControls::class);
        $owner = Identity::query()->create(['email' => 'cap-owner@example.invalid', 'password' => 'unused']);
        $service->announce(self::NODE, $owner->account_id, self::CAST, 1, 1, 0, 50);
        for ($i = 0; $i < 51; $i++) {
            $viewer = Identity::query()->create(['email' => 'cap-'.$i.'@example.invalid', 'password' => 'unused']);
            if ($i < 50) {
                $service->reserve($viewer, self::CAST, null, '127.0.0.'.($i + 1));
            } else {
                try {
                    $service->reserve($viewer, self::CAST, null, '127.0.0.99');
                    self::fail('Viewer 51 was admitted.');
                } catch (CastUnavailable) {
                    self::assertSame(50, DB::table('cast_viewer_reservations')->count());
                }
            }
        }
        $this->travel(6)->seconds();
        try {
            $service->reserve($viewer, self::CAST, null, '127.0.0.99');
            self::fail('Expired cast lease was admitted.');
        } catch (CastUnavailable) {
            self::assertSame(50, DB::table('cast_viewer_reservations')->count());
        }
        $service->announce(self::NODE, $owner->account_id, self::CAST, 1, 1, 0, 50);
        $service->reserve($viewer, self::CAST, null, '127.0.0.99');
        $service->stop(self::NODE, self::CAST);
        self::assertSame(0, DB::table('cast_viewer_reservations')->count());
    }

    public function test_concurrent_cross_cast_admissions_lock_the_global_account_budget(): void
    {
        if (DB::connection()->getDriverName() !== 'pgsql') {
            self::markTestSkipped('Cross-cast admission locking requires the PostgreSQL test wrapper (phpunit.pgsql.xml).');
        }
        $this->freezeTime();
        $service = app(CastControls::class);
        $owner = Identity::query()->create(['email' => 'race-owner@example.invalid', 'password' => 'unused']);
        $viewer = Identity::query()->create(['email' => 'race-viewer@example.invalid', 'password' => 'unused']);
        foreach (['a', 'b', 'c'] as $id) {
            $service->announce(self::NODE, $owner->account_id, str_repeat($id, 32), 1, 1, 0, 50);
        }
        $service->reserve($viewer, str_repeat('a', 32), null, '127.0.0.1');
        $processes = [];
        DB::beginTransaction();
        try {
            DB::table('cast_account_controls')->where('account_id', $viewer->account_id)->lockForUpdate()->firstOrFail();
            foreach (['b', 'c'] as $id) {
                $command = basename(PHP_BINARY) === 'frankenphp' ? [PHP_BINARY, 'php-cli'] : [PHP_BINARY];
                $command[] = base_path('tests/Support/Casts/admission_race_worker.php');
                $process = new Process($command, base_path(), null,
                    json_encode(['viewer' => $viewer->id, 'cast' => str_repeat($id, 32), 'now' => now()->toISOString()], JSON_THROW_ON_ERROR), 10);
                $process->start();
                $processes[] = $process;
            }
            $deadline = microtime(true) + 7;
            do {
                foreach ($processes as $process) {
                    $process->isRunning();
                }
                DB::select('SELECT pg_stat_clear_snapshot()');
                $waiting = (int) DB::selectOne("SELECT count(*) AS n FROM pg_stat_activity WHERE application_name='s17_cast_race' AND wait_event_type='Lock'")->n;
                if ($waiting === 2) {
                    break;
                }
                usleep(10000);
            } while (microtime(true) < $deadline);
            self::assertSame(2, $waiting, 'Both independent PostgreSQL admissions must execute and wait on the account budget. '.implode(' ', array_map(fn ($p) => $p->getErrorOutput().$p->getOutput(), $processes)));
        } finally {
            DB::rollBack();
        }
        $outcomes = [];
        foreach ($processes as $process) {
            $process->wait();
            self::assertSame(0, $process->getExitCode(), $process->getErrorOutput());
            $outcomes[] = trim($process->getOutput());
        }
        sort($outcomes);
        self::assertSame(['ADMITTED', 'DENIED'], $outcomes);
        self::assertSame(2, DB::table('cast_viewer_reservations')->where('account_id', $viewer->account_id)->count());
    }

    public function test_password_requested_at_start_is_closed_until_a_verifier_is_installed(): void
    {
        $service = app(CastControls::class);
        $owner = Identity::query()->create(['email' => 'private-owner@example.invalid', 'password' => 'unused']);
        $viewer = Identity::query()->create(['email' => 'private-viewer@example.invalid', 'password' => 'unused']);
        $service->announce(self::NODE, $owner->account_id, self::CAST, 1, 1, 0, 50, true);
        try {
            $service->reserve($viewer, self::CAST, null, '127.0.0.1');
            self::fail('A requested private cast opened before its verifier existed.');
        } catch (CastUnavailable) {
            self::assertSame(0, DB::table('cast_viewer_reservations')->count());
        }
        $service->password($owner, self::CAST, 1, 'private cast password');
        $grant = $service->reserve($viewer, self::CAST, 'private cast password', '127.0.0.1');
        self::assertSame(2,$grant->accessRevision);
    }
}
