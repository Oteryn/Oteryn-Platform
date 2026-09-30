@extends('admin.layout')

@section('title', 'Premium time')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Operations · Premium time</p>
        <h1>Premium time</h1>
        <p class="muted">Grant or revoke oteryn.premium_time for one account. Grants extend a running interval or start a new one now; revocation ends it immediately. Every change requires confirmed MFA and products.premium.manage and is audited. No payment is taken or recorded here.</p>
    </div>

    <section class="bazaar-panel" aria-labelledby="premium-search-heading">
        <div class="section-heading">
            <p class="eyebrow">Account lookup</p>
            <h2 id="premium-search-heading">Find an account</h2>
        </div>
        <form class="bazaar-admin-search" method="GET" action="{{ route('admin.premium.index') }}">
            <label for="premium-email">
                <span>Platform Identity email</span>
                <input id="premium-email" type="email" name="email" value="{{ $searchedEmail }}" required>
            </label>
            <button class="button" type="submit">Find account</button>
        </form>

        @if ($searchedEmail !== '' && ! $identity)
            <div class="empty-state"><p>No Platform Identity exists for this email address.</p></div>
        @elseif ($identity)
            <div class="bazaar-admin-wallet">
                @if ($unavailable)
                    <div class="empty-state"><p>Premium time state is unavailable for this account. No change can be made until it is repaired.</p></div>
                @else
                    <dl>
                        <div><dt>Identity</dt><dd>{{ $identity->email }}</dd></div>
                        <div><dt>Premium state</dt><dd>{{ $state }}</dd></div>
                        @if ($entitlement)
                            <div><dt>Effective from</dt><dd>{{ \App\ProductsEntitlements\Premium\PremiumTimeContract::formatTime($entitlement->effectiveFrom) }}</dd></div>
                            <div><dt>Effective until</dt><dd>{{ \App\ProductsEntitlements\Premium\PremiumTimeContract::formatTime($entitlement->effectiveUntil) }}</dd></div>
                            <div><dt>Lifecycle revision</dt><dd>{{ $entitlement->lifecycleRevision }}</dd></div>
                        @endif
                    </dl>

                    <form class="bazaar-listing-form" method="POST" action="{{ route('admin.premium.grant') }}">
                        @csrf
                        <input type="hidden" name="email" value="{{ $identity->email }}">
                        <input type="hidden" name="request_id" value="{{ \Illuminate\Support\Str::uuid() }}">
                        <label for="premium-days">
                            <span>Days to grant</span>
                            <input id="premium-days" type="number" name="duration_days" min="1" max="366" required>
                        </label>
                        <label for="premium-grant-reason">
                            <span>Grant reason</span>
                            <textarea id="premium-grant-reason" name="reason" minlength="10" maxlength="500" required></textarea>
                        </label>
                        <button class="button" type="submit">Grant Premium time</button>
                    </form>

                    @if ($state === \App\ProductsEntitlements\Premium\PremiumTimeContract::STATE_ACTIVE)
                        <form class="bazaar-listing-form" method="POST" action="{{ route('admin.premium.revoke') }}">
                            @csrf
                            <input type="hidden" name="email" value="{{ $identity->email }}">
                            <input type="hidden" name="request_id" value="{{ \Illuminate\Support\Str::uuid() }}">
                            <label for="premium-revoke-reason">
                                <span>Revocation reason</span>
                                <textarea id="premium-revoke-reason" name="reason" minlength="10" maxlength="500" required></textarea>
                            </label>
                            <button class="button" type="submit">Revoke Premium time</button>
                        </form>
                    @endif
                @endif
            </div>

            <div class="table-wrap">
                <table>
                    <thead>
                    <tr>
                        <th>Recorded</th>
                        <th>Change</th>
                        <th>Days</th>
                        <th>Revision</th>
                        <th>Effective until</th>
                    </tr>
                    </thead>
                    <tbody>
                    @forelse ($events as $event)
                        <tr>
                            <td>{{ $event['recorded_at'] }}</td>
                            <td>{{ $event['event_type'] }}</td>
                            <td>{{ $event['duration_days'] ?? '—' }}</td>
                            <td>{{ $event['lifecycle_revision'] ?? '—' }}</td>
                            <td>{{ $event['effective_until'] }}</td>
                        </tr>
                    @empty
                        <tr><td colspan="5">No Premium time changes.</td></tr>
                    @endforelse
                    </tbody>
                </table>
            </div>
        @endif
    </section>
@endsection
