@extends('admin.layout')

@section('title', 'Character Bazaar')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Operations · Character Bazaar</p>
        <h1>Character Bazaar</h1>
        <p class="muted">Review wallet history and retry only explicitly recoverable escrow or settlement operations. All actions require confirmed MFA and marketplace.manage.</p>
    </div>

    <section class="bazaar-panel" aria-labelledby="wallet-search-heading">
        <div class="section-heading">
            <p class="eyebrow">Wallet inspection</p>
            <h2 id="wallet-search-heading">Find a wallet</h2>
        </div>
        <form class="bazaar-admin-search" method="GET" action="{{ route('admin.marketplace.index') }}">
            <label for="wallet-email">
                <span>Platform Identity email</span>
                <input id="wallet-email" type="email" name="email" value="{{ $searchedEmail }}" required>
            </label>
            <button class="button" type="submit">Find wallet</button>
        </form>

        @if ($searchedEmail !== '' && ! $identity)
            <div class="empty-state"><p>No Platform Identity exists for this email address.</p></div>
        @elseif ($identity)
            <div class="bazaar-admin-wallet">
                <dl>
                    <div><dt>Identity</dt><dd>{{ $identity->email }}</dd></div>
                    <div><dt>Available</dt><dd>{{ number_format($wallet?->available_balance ?? 0, 0, ',', ' ') }} Oteryn Coins</dd></div>
                    <div><dt>Reserved</dt><dd>{{ number_format($wallet?->reserved_balance ?? 0, 0, ',', ' ') }} Oteryn Coins</dd></div>
                </dl>

                <form class="bazaar-listing-form" method="POST" action="{{ route('admin.marketplace.wallet.adjust') }}">
                    @csrf
                    <input type="hidden" name="email" value="{{ $identity->email }}">
                    <input type="hidden" name="request_id" value="{{ \Illuminate\Support\Str::uuid() }}">
                    <label for="wallet-amount">
                        <span>Signed adjustment</span>
                        <input id="wallet-amount" type="number" name="amount" min="-1000000000" max="1000000000" required>
                    </label>
                    <label for="wallet-reason">
                        <span>Operational reason</span>
                        <textarea id="wallet-reason" name="reason" minlength="10" maxlength="500" required></textarea>
                    </label>
                    <button class="button" type="submit">Record adjustment</button>
                </form>
            </div>

            <div class="table-wrap">
                <table>
                    <thead>
                    <tr>
                        <th>Time</th>
                        <th>Operation</th>
                        <th>Available Δ</th>
                        <th>Reserved Δ</th>
                        <th>Auction</th>
                    </tr>
                    </thead>
                    <tbody>
                    @forelse ($ledger as $entry)
                        <tr>
                            <td>{{ $entry->created_at->utc()->format('Y-m-d H:i:s') }} UTC</td>
                            <td>{{ $entry->operation_type }}</td>
                            <td>{{ number_format($entry->available_delta, 0, ',', ' ') }}</td>
                            <td>{{ number_format($entry->reserved_delta, 0, ',', ' ') }}</td>
                            <td>{{ $entry->auction_id ?? '—' }}</td>
                        </tr>
                    @empty
                        <tr><td colspan="5">No wallet ledger entries.</td></tr>
                    @endforelse
                    </tbody>
                </table>
            </div>
        @endif
    </section>

    <section class="bazaar-panel" aria-labelledby="recovery-heading">
        <div class="section-heading">
            <p class="eyebrow">Recovery queue</p>
            <h2 id="recovery-heading">Auctions requiring operator reconciliation</h2>
            <p class="muted">Retry reads the actual Canary owner and append-only wallet ledger before deciding the next idempotent step.</p>
        </div>

        <div class="table-wrap">
            <table>
                <thead>
                <tr>
                    <th>Auction</th>
                    <th>Character</th>
                    <th>Saga state</th>
                    <th>Failure code</th>
                    <th>Updated</th>
                    <th><span class="sr-only">Actions</span></th>
                </tr>
                </thead>
                <tbody>
                @forelse ($recoveryAuctions as $auction)
                    <tr>
                        <td>#{{ $auction->id }}</td>
                        <td>{{ $auction->player_name }}</td>
                        <td>{{ $auction->saga_state }}</td>
                        <td>{{ $auction->failure_code ?? '—' }}</td>
                        <td>{{ $auction->updated_at->utc()->format('Y-m-d H:i:s') }} UTC</td>
                        <td>
                            <form method="POST" action="{{ route('admin.marketplace.auctions.recover', $auction) }}">
                                @csrf
                                <button class="button button-danger" type="submit">Run bounded recovery</button>
                            </form>
                        </td>
                    </tr>
                @empty
                    <tr><td colspan="6">No auctions require operator recovery.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>
@endsection
