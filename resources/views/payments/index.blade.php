@extends('identity.layout')

@section('title', __('payments.title'))

@section('content')
    <header class="page-header">
        <p class="eyebrow">{{ __('payments.eyebrow') }}</p>
        <h1>{{ __('payments.heading') }}</h1>
        <p class="muted">{{ __('payments.intro') }}</p>
        @include('identity.partials.locale-switcher', [
            'localeRoute' => 'payments.account.index',
            'localeParameters' => [],
        ])
    </header>

    <section class="panel" aria-labelledby="payment-history-heading">
        <div class="page-header">
            <p class="eyebrow">{{ __('payments.history') }}</p>
            <h2 id="payment-history-heading">{{ __('payments.history') }}</h2>
            <p class="muted">{{ __('payments.history_help') }}</p>
        </div>

        @if ($orders->isEmpty())
            <div class="empty-state">
                <p>{{ __('payments.empty') }}</p>
            </div>
        @else
            <div class="table-wrap">
                <table>
                    <thead>
                    <tr>
                        <th>{{ __('payments.created') }}</th>
                        <th>{{ __('payments.order') }}</th>
                        <th>{{ __('payments.amount') }}</th>
                        <th>{{ __('payments.status') }}</th>
                        <th><span class="sr-only">{{ __('payments.details') }}</span></th>
                    </tr>
                    </thead>
                    <tbody>
                    @foreach ($orders as $order)
                        @php
                            $whole = intdiv($order->amount_minor, 100);
                            $fraction = str_pad((string) ($order->amount_minor % 100), 2, '0', STR_PAD_LEFT);
                            $separator = app()->getLocale() === 'pl' ? ',' : '.';
                            $formattedAmount = number_format($whole, 0, '', ' ').$separator.$fraction;
                        @endphp
                        <tr>
                            <td data-label="{{ __('payments.created') }}">
                                <time datetime="{{ $order->created_at->toAtomString() }}">{{ $order->created_at->utc()->format('Y-m-d H:i') }} UTC</time>
                            </td>
                            <td data-label="{{ __('payments.order') }}"><code>{{ $order->public_id }}</code></td>
                            <td data-label="{{ __('payments.amount') }}">{{ $formattedAmount }} {{ $order->currency }}</td>
                            <td data-label="{{ __('payments.status') }}">{{ __('payments.status_labels.'.$order->status) }}</td>
                            <td>
                                <a class="button button-secondary" href="{{ route('payments.account.return', ['orderPublicId' => $order->public_id]) }}">
                                    {{ __('payments.details') }}
                                </a>
                            </td>
                        </tr>
                    @endforeach
                    </tbody>
                </table>
            </div>
            {{ $orders->withQueryString()->links() }}
        @endif
    </section>

    <section class="panel" aria-labelledby="payment-test-heading">
        <div class="page-header">
            <p class="eyebrow">{{ __('payments.test.eyebrow') }}</p>
            <h2 id="payment-test-heading">{{ __('payments.test.heading') }}</h2>
            <p class="muted">{{ __('payments.test.intro') }}</p>
        </div>

        @if ($testSurfaceAvailable && $testAmountMinor !== null && $allowedCurrencies !== [])
            @php
                $testWhole = intdiv($testAmountMinor, 100);
                $testFraction = str_pad((string) ($testAmountMinor % 100), 2, '0', STR_PAD_LEFT);
                $testSeparator = app()->getLocale() === 'pl' ? ',' : '.';
                $testAmount = number_format($testWhole, 0, '', ' ').$testSeparator.$testFraction;
            @endphp
            <form method="POST" action="{{ route('payments.account.test-checkout.store') }}" class="stacked-form">
                @csrf
                <input type="hidden" name="request_id" value="{{ \Illuminate\Support\Str::uuid() }}">
                <label for="payment-test-currency">
                    <span>{{ __('payments.test.currency') }}</span>
                    <select id="payment-test-currency" name="currency" required>
                        @foreach ($allowedCurrencies as $currency)
                            <option value="{{ $currency }}">{{ $currency }}</option>
                        @endforeach
                    </select>
                </label>
                <p>{{ __('payments.test.amount', ['amount' => $testAmount, 'currency' => $allowedCurrencies[0]]) }}</p>
                <button type="submit">{{ __('payments.test.submit') }}</button>
            </form>
        @else
            <div class="notice" role="status">
                {{ __('payments.test.unavailable') }}
            </div>
        @endif
    </section>
@endsection
