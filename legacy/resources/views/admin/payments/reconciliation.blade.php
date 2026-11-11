@extends('admin.layout')

@section('title', __('payments.admin.title'))

@section('content')
    <header class="page-header">
        <p class="eyebrow">{{ __('payments.admin.eyebrow') }}</p>
        <h1>{{ __('payments.admin.heading') }}</h1>
        <p class="muted">{{ __('payments.admin.intro') }}</p>
        @include('identity.partials.locale-switcher', [
            'localeRoute' => 'admin.payments.reconciliation.index',
            'localeParameters' => [],
        ])
    </header>

    @unless ($testSurfaceAvailable)
        <div class="notice alert-warning" role="status">
            {{ __('payments.admin.read_only') }}
        </div>
    @endunless

    <section class="panel" aria-labelledby="payment-reconciliation-queue">
        <div class="page-header">
            <p class="eyebrow">{{ __('payments.admin.queue') }}</p>
            <h2 id="payment-reconciliation-queue">{{ __('payments.admin.queue') }}</h2>
            <p class="muted">{{ __('payments.admin.queue_help') }}</p>
        </div>

        @if ($entries->isEmpty())
            <div class="empty-state"><p>{{ __('payments.admin.empty') }}</p></div>
        @else
            <div class="table-wrap">
                <table>
                    <thead>
                    <tr>
                        <th>{{ __('payments.created') }}</th>
                        <th>{{ __('payments.order') }}</th>
                        <th>{{ __('payments.admin.issue') }}</th>
                        <th>{{ __('payments.admin.provider_event') }}</th>
                        <th>{{ __('payments.status') }}</th>
                        <th>{{ __('payments.admin.resolution') }}</th>
                        <th>{{ __('payments.admin.actions') }}</th>
                    </tr>
                    </thead>
                    <tbody>
                    @foreach ($entries as $entry)
                        @php($resolution = $entry->resolutions->last())
                        <tr>
                            <td data-label="{{ __('payments.created') }}">
                                <time datetime="{{ $entry->created_at->toAtomString() }}">{{ $entry->created_at->utc()->format('Y-m-d H:i:s') }} UTC</time>
                            </td>
                            <td data-label="{{ __('payments.order') }}">
                                @if ($entry->paymentOrder)
                                    <code>{{ $entry->paymentOrder->public_id }}</code>
                                @else
                                    {{ __('payments.admin.unmatched_order') }}
                                @endif
                            </td>
                            <td data-label="{{ __('payments.admin.issue') }}"><code>{{ $entry->issue_type }}</code></td>
                            <td data-label="{{ __('payments.admin.provider_event') }}">
                                @if ($entry->providerEvent)
                                    <code>{{ $entry->providerEvent->event_type }}</code>
                                    @if ($entry->providerEvent->failure_code)
                                        <br><span class="muted"><code>{{ $entry->providerEvent->failure_code }}</code></span>
                                    @endif
                                @else
                                    {{ __('payments.admin.no_event') }}
                                @endif
                            </td>
                            <td data-label="{{ __('payments.status') }}">
                                <span class="badge {{ $entry->state === \App\Payments\Models\PaymentReconciliationEntry::STATE_OPEN ? 'badge-warning' : 'badge-success' }}">
                                    {{ $entry->state === \App\Payments\Models\PaymentReconciliationEntry::STATE_OPEN ? __('payments.admin.open') : __('payments.admin.resolved_state') }}
                                </span>
                            </td>
                            <td data-label="{{ __('payments.admin.resolution') }}">
                                @if ($resolution)
                                    {{ __('payments.admin.resolution_labels.'.$resolution->resolution_code) }}
                                    <br><time class="muted" datetime="{{ $resolution->created_at->toAtomString() }}">{{ $resolution->created_at->utc()->format('Y-m-d H:i:s') }} UTC</time>
                                @else
                                    —
                                @endif
                            </td>
                            <td data-label="{{ __('payments.admin.actions') }}">
                                @if ($entry->state === \App\Payments\Models\PaymentReconciliationEntry::STATE_OPEN && $testSurfaceAvailable)
                                    <form method="POST" action="{{ route('admin.payments.reconciliation.resolve', ['reconciliation' => $entry->id]) }}">
                                        @csrf
                                        <input type="hidden" name="request_id" value="{{ \Illuminate\Support\Str::uuid() }}">
                                        <input type="hidden" name="resolution_code" value="reviewed_no_payment_state_change">
                                        <button class="button button-secondary" type="submit">{{ __('payments.admin.resolve') }}</button>
                                        <p class="muted">{{ __('payments.admin.resolve_help') }}</p>
                                    </form>
                                @else
                                    —
                                @endif
                            </td>
                        </tr>
                    @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>
@endsection
