@extends('admin.layout')

@section('title', __('support.title.enforcement_detail'))

@section('content')
    <div class="section-heading">
        <div><p class="eyebrow">{{ $record->public_id }}</p><h1>{{ __('support.title.enforcement_detail') }}</h1></div>
        <a class="button" href="{{ route('admin.moderation.enforcement.edit', ['enforcementRecord' => $record, 'locale' => app()->getLocale()]) }}">{{ __('support.enforcement.edit') }}</a>
    </div>

    <dl class="detail-grid">
        <div><dt>{{ __('support.common.identity_id') }}</dt><dd>#{{ $record->identity_id }} · {{ $record->identity?->email }}</dd></div>
        <div><dt>{{ __('support.common.category') }}</dt><dd>{{ __('support.category.'.$record->category) }}</dd></div>
        <div><dt>{{ __('support.common.status') }}</dt><dd>{{ __('support.enforcement_status.'.$record->status) }}</dd></div>
        <div><dt>{{ __('support.enforcement.effective_at') }}</dt><dd>{{ $record->effective_at->toDateTimeString() }}</dd></div>
        <div><dt>{{ __('support.enforcement.expires_at') }}</dt><dd>{{ $record->expires_at?->toDateTimeString() ?? __('support.common.none') }}</dd></div>
        <div><dt>{{ __('support.enforcement.acknowledged_at') }}</dt><dd>{{ $record->acknowledged_at?->toDateTimeString() ?? __('support.common.none') }}</dd></div>
        <div><dt>{{ __('support.enforcement.appeal_status') }}</dt><dd>{{ __('support.appeal_status.'.$record->appeal_status) }}</dd></div>
    </dl>

    <h2>{{ __('support.enforcement.public_reason') }}</h2>
    <p class="pre-wrap">{{ $record->public_reason }}</p>

    @if ($record->moderator_notes)
        <h2>{{ __('support.enforcement.moderator_notes') }}</h2>
        <p class="pre-wrap">{{ $record->moderator_notes }}</p>
    @endif

    @if ($record->appeal_message)
        <h2>{{ __('support.enforcement.appeal_message') }}</h2>
        <p class="pre-wrap">{{ $record->appeal_message }}</p>
        <form method="POST" action="{{ route('admin.moderation.enforcement.appeal', ['enforcementRecord' => $record, 'locale' => app()->getLocale()]) }}" class="form-stack">
            @csrf
            @method('PUT')
            <input type="hidden" name="lock_version" value="{{ $record->lock_version }}">

            <label for="appeal_status">{{ __('support.enforcement.appeal_status') }}</label>
            <select id="appeal_status" name="appeal_status" required>
                @foreach (['reviewing', 'accepted', 'rejected'] as $status)
                    <option value="{{ $status }}" @selected(old('appeal_status', $record->appeal_status) === $status)>{{ __('support.appeal_status.'.$status) }}</option>
                @endforeach
            </select>

            <label for="appeal_outcome">{{ __('support.enforcement.appeal_outcome') }}</label>
            <textarea id="appeal_outcome" name="appeal_outcome" rows="7" maxlength="4000">{{ old('appeal_outcome', $record->appeal_outcome) }}</textarea>
            <button class="button" type="submit">{{ __('support.common.save') }}</button>
        </form>
    @endif

    <a class="button button-secondary" href="{{ route('admin.moderation.enforcement.index', ['locale' => app()->getLocale()]) }}">{{ __('support.common.back') }}</a>
@endsection
