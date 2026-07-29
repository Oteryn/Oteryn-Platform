@extends('identity.layout')

@section('title', __('support.title.enforcement_detail'))

@section('content')
    @include('support.partials.navigation')
    @include('identity.partials.locale-switcher', [
        'localeRoute' => 'support.enforcement.show',
        'localeParameters' => ['enforcementRecord' => $record],
    ])

    <p class="eyebrow">{{ $record->public_id }}</p>
    <h1>{{ __('support.title.enforcement_detail') }}</h1>
    <p class="supporting-copy">{{ __('support.enforcement.platform_only') }}</p>

    <dl class="detail-grid">
        <div><dt>{{ __('support.common.category') }}</dt><dd>{{ __('support.category.'.$record->category) }}</dd></div>
        <div><dt>{{ __('support.common.status') }}</dt><dd>{{ __('support.enforcement_status.'.$record->status) }}</dd></div>
        <div><dt>{{ __('support.enforcement.effective_at') }}</dt><dd>{{ $record->effective_at->toDateTimeString() }}</dd></div>
        <div><dt>{{ __('support.enforcement.expires_at') }}</dt><dd>{{ $record->expires_at?->toDateTimeString() ?? __('support.common.none') }}</dd></div>
        <div><dt>{{ __('support.enforcement.acknowledged_at') }}</dt><dd>{{ $record->acknowledged_at?->toDateTimeString() ?? __('support.common.none') }}</dd></div>
        <div><dt>{{ __('support.enforcement.appeal_status') }}</dt><dd>{{ __('support.appeal_status.'.$record->appeal_status) }}</dd></div>
    </dl>

    <h2>{{ __('support.enforcement.public_reason') }}</h2>
    <p class="pre-wrap">{{ $record->public_reason }}</p>

    @if ($record->appeal_message)
        <h2>{{ __('support.enforcement.appeal_message') }}</h2>
        <p class="pre-wrap">{{ $record->appeal_message }}</p>
    @endif
    @if ($record->appeal_outcome)
        <h2>{{ __('support.enforcement.appeal_outcome') }}</h2>
        <p class="pre-wrap">{{ $record->appeal_outcome }}</p>
    @endif

    @if ($record->acknowledged_at === null)
        <form method="POST" action="{{ route('support.enforcement.acknowledge', ['enforcementRecord' => $record, 'locale' => app()->getLocale()]) }}" class="action-row">
            @csrf
            <input type="hidden" name="lock_version" value="{{ $record->lock_version }}">
            <button class="button" type="submit">{{ __('support.enforcement.acknowledge') }}</button>
        </form>
    @endif

    @if (in_array($record->appeal_status, ['none', 'rejected'], true))
        <form method="POST" action="{{ route('support.enforcement.appeal', ['enforcementRecord' => $record, 'locale' => app()->getLocale()]) }}" class="form-stack">
            @csrf
            <input type="hidden" name="lock_version" value="{{ $record->lock_version }}">
            <label for="appeal_message">{{ __('support.enforcement.appeal_message') }}</label>
            <textarea id="appeal_message" name="appeal_message" rows="8" maxlength="4000" required>{{ old('appeal_message') }}</textarea>
            <button class="button" type="submit">{{ __('support.enforcement.appeal') }}</button>
        </form>
    @endif

    <a class="button button-secondary" href="{{ route('support.enforcement.index', ['locale' => app()->getLocale()]) }}">{{ __('support.common.back') }}</a>
@endsection
