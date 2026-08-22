@extends('identity.layout')

@section('title', __('payments.return.heading'))

@section('content')
    @php
        $whole = intdiv($order->amount_minor, 100);
        $fraction = str_pad((string) ($order->amount_minor % 100), 2, '0', STR_PAD_LEFT);
        $separator = app()->getLocale() === 'pl' ? ',' : '.';
        $formattedAmount = number_format($whole, 0, '', ' ').$separator.$fraction;
    @endphp

    <header class="page-header">
        <p class="eyebrow">{{ __('payments.return.eyebrow') }}</p>
        <h1>{{ __('payments.return.heading') }}</h1>
        <p class="muted">{{ __('payments.return.intro') }}</p>
        @include('identity.partials.locale-switcher', [
            'localeRoute' => 'payments.account.return',
            'localeParameters' => ['orderPublicId' => $order->public_id],
        ])
    </header>

    <section class="card" aria-labelledby="payment-current-state">
        <p class="eyebrow">{{ __('payments.return.current') }}</p>
        <h2 id="payment-current-state">{{ __('payments.status_labels.'.$order->status) }}</h2>
        <dl>
            <dt>{{ __('payments.order') }}</dt>
            <dd><code>{{ $order->public_id }}</code></dd>
            <dt>{{ __('payments.amount') }}</dt>
            <dd>{{ $formattedAmount }} {{ $order->currency }}</dd>
            <dt>{{ __('payments.status') }}</dt>
            <dd>{{ __('payments.status_labels.'.$order->status) }}</dd>
        </dl>
        <div class="notice alert-warning" role="status">
            {{ __('payments.return.intro') }}
        </div>
        <div class="action-row">
            <a class="button button-secondary" href="{{ route('payments.account.index') }}">
                {{ __('payments.return.back') }}
            </a>
        </div>
    </section>
@endsection
