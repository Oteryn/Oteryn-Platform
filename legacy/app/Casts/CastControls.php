<?php

namespace App\Casts;

use App\Identity\Models\Identity;
use Illuminate\Hashing\Argon2IdHasher;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\RateLimiter;
use Illuminate\Support\Str;

/**
 * Account-only admission storage. Node-facing calls require an authenticated node adapter.
 * Row shapes describe the typed columns created by migration 0968, not request data.
 *
 * @phpstan-type PolicyRow \stdClass&object{cast_id: string, node_id: string, owner_account_id: string, epoch: int, access_revision: int, delay_seconds: int, viewer_cap: int, password_hash: string|null, password_required: bool, expires_at: string}
 * @phpstan-type ReservationRow \stdClass&object{reservation_id: string, cast_id: string, account_id: string, security_generation: int, access_revision: int, consumed: bool, chat_selected: bool, expires_at: string}
 */
final class CastControls
{
    public function announce(string $node, string $owner, string $cast, int $epoch, int $revision, int $delay, int $cap, bool $passwordRequired = false): void
    {
        if (! Str::isUuid($node) || ! Str::isUuid($owner) || ! preg_match('/\A[0-9a-f]{32}\z/D', $cast)
            || $epoch < 1 || $revision < 1 || $delay < 0 || $delay > 300 || $cap < 1 || $cap > 50) {
            throw new CastUnavailable;
        }
        DB::transaction(function () use ($node, $owner, $cast, $epoch, $revision, $delay, $cap, $passwordRequired): void {
            // The account lock also serializes initially absent policy rows.
            $identity = Identity::query()->where('account_id', $owner)->first();
            if (! $identity) {
                throw new CastUnavailable;
            }
            $this->active($identity);
            $this->accountLock($owner);
            $old = $this->policy($cast, true);
            if ($old && ($old->node_id !== $node || $old->owner_account_id !== $owner
                || $epoch < $old->epoch || $revision !== (int) $old->access_revision)) {
                throw new CastUnavailable;
            }
            if ($old && $epoch !== (int) $old->epoch) {
                DB::table('cast_viewer_reservations')->where('cast_id', $cast)->delete();
            }
            DB::table('cast_access_policies')->updateOrInsert(['cast_id' => $cast], [
                'node_id' => $node, 'owner_account_id' => $owner, 'epoch' => $epoch,
                'access_revision' => $revision, 'delay_seconds' => $delay, 'viewer_cap' => $cap,
                'password_required' => $passwordRequired || ($old !== null && $old->password_hash !== null),
                'expires_at' => now()->addSeconds(5),
            ]);
        }, 3);
    }

    public function password(Identity $owner, string $cast, int $expectedRevision, #[\SensitiveParameter] ?string $password): int
    {
        $this->active($owner);
        if ($expectedRevision < 1 || $expectedRevision === PHP_INT_MAX) {
            throw new CastUnavailable;
        }
        if ($password !== null && (strlen($password) < 1 || strlen($password) > 128)) {
            throw new CastUnavailable;
        }
        $this->budget('owner:'.$owner->account_id, 5);
        $hash = $password === null ? null : $this->hasher()->make($password);

        return DB::transaction(function () use ($owner, $cast, $expectedRevision, $hash): int {
            $current = Identity::query()->whereKey($owner->id)->firstOrFail();
            $this->active($current);
            if ($current->native_security_generation !== $owner->native_security_generation) {
                throw new CastUnavailable;
            }
            $policy = $this->policy($cast, true);
            if (! $policy || $policy->owner_account_id !== $owner->account_id
                || (int) $policy->access_revision !== $expectedRevision || now()->gte($policy->expires_at)) {
                throw new CastUnavailable;
            }
            $revision = $expectedRevision + 1;
            DB::table('cast_access_policies')->where('cast_id', $cast)->update([
                'password_hash' => $hash, 'password_required' => $hash !== null, 'access_revision' => $revision,
            ]);
            DB::table('cast_viewer_reservations')->where('cast_id', $cast)->delete();

            return $revision;
        }, 3);
    }

    public function reserve(?Identity $viewer, string $cast, #[\SensitiveParameter] ?string $password, string $ip, bool $chat = false): CastReservation
    {
        if ($viewer === null) {
            throw new CastUnavailable;
        }
        $this->active($viewer);
        $this->budget('account:'.$viewer->account_id, 5);
        $this->budget('ip:'.hash('sha256', $ip), 20);
        if ($password !== null && strlen($password) > 128) {
            throw new CastUnavailable;
        }
        // Hash work is outside the transaction. Recheck the exact policy after acquiring locks.
        $candidate = $this->policy($cast);
        if (! $candidate || now()->gte($candidate->expires_at)
            || ($candidate->password_required && $candidate->password_hash === null)
            || ($candidate->password_hash !== null && ($password === null
                || ! $this->hasher()->check($password, $candidate->password_hash)))) {
            throw new CastUnavailable;
        }

        return DB::transaction(function () use ($viewer, $cast, $candidate, $chat): CastReservation {
            $this->accountLock($viewer->account_id);
            $current = Identity::query()->whereKey($viewer->id)->firstOrFail();
            $this->active($current);
            if ($current->native_security_generation !== $viewer->native_security_generation) {
                throw new CastUnavailable;
            }
            $policy = $this->policy($cast, true);
            if (! $policy || now()->gte($policy->expires_at)
                || $policy->password_required !== $candidate->password_required
                || $policy->password_hash !== $candidate->password_hash
                || $policy->access_revision !== $candidate->access_revision || $policy->epoch !== $candidate->epoch) {
                throw new CastUnavailable;
            }
            DB::table('cast_viewer_reservations')->where('cast_id', $cast)->where('expires_at', '<=', now())->delete();
            DB::table('cast_viewer_reservations')->where('account_id', $viewer->account_id)->where('expires_at', '<=', now())->delete();
            $accountCount = DB::table('cast_viewer_reservations')->where('account_id', $viewer->account_id)
                ->where('expires_at', '>', now())->count();
            $castCount = DB::table('cast_viewer_reservations')->where('cast_id', $cast)
                ->where('expires_at', '>', now())->count();
            if ($accountCount >= 2 || $castCount >= $policy->viewer_cap) {
                throw new CastUnavailable;
            }
            $id = (string) Str::uuid7();
            $expiry = now()->addSeconds(5);
            $ticket = 'ocv1.'.bin2hex(random_bytes(32));
            DB::table('cast_viewer_reservations')->insert([
                'reservation_id' => $id, 'ticket_hash' => hash('sha256', $ticket), 'cast_id' => $cast, 'account_id' => $viewer->account_id,
                'security_generation' => $current->native_security_generation, 'chat_selected' => $chat,
                'access_revision' => $policy->access_revision, 'consumed' => false, 'expires_at' => $expiry,
            ]);

            return new CastReservation($id, $viewer->account_id, $cast, $policy->node_id,
                (int) $policy->epoch, (int) $policy->access_revision, $expiry->getTimestamp(), $chat, $ticket);
        }, 3);
    }

    public function consumeTicket(string $node, #[\SensitiveParameter] string $ticket): CastReservation
    {
        if (! preg_match('/\Aocv1\.[0-9a-f]{64}\z/D', $ticket)) {
            throw new CastUnavailable;
        }
        $row = $this->reservation(hash('sha256', $ticket), true);
        if (! $row) {
            throw new CastUnavailable;
        }

        return $this->consume($node, $row->reservation_id);
    }

    /** Single-use consume; renewals cannot turn a different node or revoked account into a viewer. */
    public function consume(string $node, string $reservation, bool $renew = false): CastReservation
    {
        return DB::transaction(function () use ($node, $reservation, $renew): CastReservation {
            $hint = $this->reservation($reservation);
            if (! $hint) {
                throw new CastUnavailable;
            }
            $this->accountLock($hint->account_id);
            $policy = $this->policy($hint->cast_id, true);
            $row = $this->reservation($reservation, false, true);
            $identity = Identity::query()->where('account_id', $hint->account_id)->first();
            if (! $identity) {
                throw new CastUnavailable;
            }
            $this->active($identity);
            if (! $row || ! $policy || $policy->node_id !== $node || now()->gte($row->expires_at)
                || now()->gte($policy->expires_at) || (bool) $row->consumed !== $renew
                || $row->security_generation !== $identity->native_security_generation
                || $row->access_revision !== $policy->access_revision) {
                throw new CastUnavailable;
            }
            $expiry = now()->addSeconds(5);
            DB::table('cast_viewer_reservations')->where('reservation_id', $reservation)->update([
                'consumed' => true, 'expires_at' => $expiry,
            ]);

            return new CastReservation($reservation, $row->account_id, $row->cast_id, $node,
                (int) $policy->epoch, (int) $policy->access_revision, $expiry->getTimestamp(), (bool) $row->chat_selected);
        }, 3);
    }

    /** Release is idempotent and node-bound; kick uses the same operation. */
    public function release(string $node, string $reservation): void
    {
        DB::table('cast_viewer_reservations')->where('reservation_id', $reservation)
            ->whereIn('cast_id', DB::table('cast_access_policies')->select('cast_id')->where('node_id', $node))->delete();
    }

    public function stop(string $node, string $cast): void
    {
        DB::table('cast_access_policies')->where('cast_id', $cast)->where('node_id', $node)->delete();
    }

    /** @phpstan-return PolicyRow|null */
    private function policy(string $cast, bool $lock = false): ?\stdClass
    {
        $query = DB::table('cast_access_policies')->where('cast_id', $cast);
        if ($lock) {
            $query->lockForUpdate();
        }
        /** @var PolicyRow|null $row */
        $row = $query->first();

        return $row;
    }

    /** @phpstan-return ReservationRow|null */
    private function reservation(string $value, bool $ticket = false, bool $lock = false): ?\stdClass
    {
        $query = DB::table('cast_viewer_reservations')->where($ticket ? 'ticket_hash' : 'reservation_id', $value);
        if ($lock) {
            $query->lockForUpdate();
        }
        /** @var ReservationRow|null $row */
        $row = $query->first();

        return $row;
    }

    private function hasher(): Argon2IdHasher
    {
        $hasher = Hash::driver('argon2id');
        if (! $hasher instanceof Argon2IdHasher) {
            throw new CastUnavailable;
        }

        return $hasher;
    }

    private function accountLock(string $account): void
    {
        DB::table('cast_account_controls')->insertOrIgnore(['account_id' => $account]);
        DB::table('cast_account_controls')->where('account_id', $account)->lockForUpdate()->firstOrFail();
    }

    private function active(Identity $identity): void
    {
        if (! $identity->exists || $identity->disabled_at !== null || $identity->terminated_at !== null) {
            throw new CastUnavailable;
        }
    }

    private function budget(string $subject, int $limit): void
    {
        $key = 'cast-admission:'.$subject;
        if (RateLimiter::hit($key, 60) > $limit) {
            throw new CastUnavailable;
        }
    }
}
