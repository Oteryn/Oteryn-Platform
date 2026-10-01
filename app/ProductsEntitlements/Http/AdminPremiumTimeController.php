<?php

namespace App\ProductsEntitlements\Http;

use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalEmail;
use App\ProductsEntitlements\Premium\PremiumSnapshotUnavailable;
use App\ProductsEntitlements\Premium\PremiumTimeContract;
use App\ProductsEntitlements\Premium\PremiumTimeEntitlement;
use App\ProductsEntitlements\Premium\PremiumTimeException;
use App\ProductsEntitlements\Premium\PremiumTimeLedger;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;

/**
 * Operator surface for oteryn.premium_time (contract 2.5). Route middleware enforces the confirmed-MFA session and
 * the exact products.premium.manage permission; PremiumTimeLedger owns validation, locking, idempotency and audit.
 */
final class AdminPremiumTimeController
{
    public function __construct(private readonly PremiumTimeLedger $ledger) {}

    public function index(Request $request): View
    {
        $email = $request->query('email');
        $identity = is_string($email) && $email !== ''
            ? Identity::query()->where('email', CanonicalEmail::normalize($email))->first()
            : null;

        $entitlement = null;
        $state = PremiumTimeContract::STATE_NONE;
        $unavailable = false;
        if ($identity instanceof Identity) {
            try {
                $entitlement = PremiumTimeEntitlement::forIdentity($identity->id);
                $state = $entitlement?->classify(now()->getTimestamp()) ?? PremiumTimeContract::STATE_NONE;
            } catch (PremiumSnapshotUnavailable) {
                $unavailable = true;
            }
        }

        return view('admin.premium.index', [
            'searchedEmail' => is_string($email) ? $email : '',
            'identity' => $identity,
            'entitlement' => $entitlement,
            'state' => $state,
            'unavailable' => $unavailable,
            'events' => $identity instanceof Identity ? $this->events($identity->id) : [],
        ]);
    }

    public function grant(Request $request): RedirectResponse
    {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        /** @var array{account_id: string, duration_days: int, reason: string, request_id: string} $validated */
        $validated = $request->validate([
            'account_id' => ['required', 'string', 'size:36'],
            'duration_days' => ['required', 'integer', 'min:'.PremiumTimeContract::MIN_GRANT_DAYS, 'max:'.PremiumTimeContract::MAX_GRANT_DAYS],
            'reason' => ['required', 'string', 'min:10', 'max:500'],
            'request_id' => ['required', 'uuid'],
        ]);

        $target = $this->target($validated['account_id']);
        if (! $target instanceof Identity) {
            return back()->withInput()->withErrors(['account_id' => 'No Platform Identity exists for this AccountId.']);
        }

        try {
            $this->ledger->grant($actor, $target, (int) $validated['duration_days'], $validated['reason'], $validated['request_id']);
        } catch (PremiumTimeException $exception) {
            return back()->withInput()->withErrors(['premium' => $exception->getMessage()]);
        }

        return redirect()->route('admin.premium.index', ['email' => $target->email])
            ->with('status', 'Premium time granted.');
    }

    public function revoke(Request $request): RedirectResponse
    {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        /** @var array{account_id: string, reason: string, request_id: string} $validated */
        $validated = $request->validate([
            'account_id' => ['required', 'string', 'size:36'],
            'reason' => ['required', 'string', 'min:10', 'max:500'],
            'request_id' => ['required', 'uuid'],
        ]);

        $target = $this->target($validated['account_id']);
        if (! $target instanceof Identity) {
            return back()->withInput()->withErrors(['account_id' => 'No Platform Identity exists for this AccountId.']);
        }

        try {
            $this->ledger->revoke($actor, $target, $validated['reason'], $validated['request_id']);
        } catch (PremiumTimeException $exception) {
            return back()->withInput()->withErrors(['premium' => $exception->getMessage()]);
        }

        return redirect()->route('admin.premium.index', ['email' => $target->email])
            ->with('status', 'Premium time revoked.');
    }

    /** Mutations bind to the immutable AccountId the operator reviewed, never to a reassignable email address. */
    private function target(string $accountId): ?Identity
    {
        return PremiumTimeContract::isUuidV7($accountId)
            ? Identity::query()->where('account_id', $accountId)->first()
            : null;
    }

    /**
     * @return list<array{event_type: string, duration_days: ?int, lifecycle_revision: ?int, effective_until: string, recorded_at: string}>
     */
    private function events(int $identityId): array
    {
        $events = [];
        foreach (DB::table('premium_time_entitlement_events')->where('identity_id', $identityId)->orderByDesc('id')->limit(20)->get() as $row) {
            $until = PremiumTimeContract::databaseInteger($row->effective_until);
            $events[] = [
                'event_type' => is_string($row->event_type) ? $row->event_type : '',
                'duration_days' => $row->duration_days === null ? null : PremiumTimeContract::databaseInteger($row->duration_days),
                'lifecycle_revision' => PremiumTimeContract::databaseInteger($row->lifecycle_revision),
                'effective_until' => $until === null ? '' : PremiumTimeContract::formatTime($until),
                'recorded_at' => is_string($row->created_at) ? $row->created_at : '',
            ];
        }

        return $events;
    }
}
