@extends('admin.layout')

@section('title', $record ? __('support.title.admin_enforcement_edit') : __('support.title.admin_enforcement_create'))

@section('content')
    <p class="eyebrow">{{ __('support.nav.admin_enforcement') }}</p>
    <h1>{{ $record ? __('support.title.admin_enforcement_edit') : __('support.title.admin_enforcement_create') }}</h1>
    <p class="supporting-copy">{{ __('support.enforcement.platform_only') }}</p>

    <form method="POST" action="{{ $record
        ? route('admin.moderation.enforcement.update', ['enforcementRecord' => $record, 'locale' => app()->getLocale()])
        : route('admin.moderation.enforcement.store', ['locale' => app()->getLocale()]) }}" class="form-stack">
        @csrf
        @if ($record) @method('PUT') @endif
        @if ($record)<input type="hidden" name="lock_version" value="{{ $record->lock_version }}">@endif

        <label for="target_identity_id">{{ __('support.common.identity_id') }}</label>
        <input id="target_identity_id" name="target_identity_id" type="number" min="1" required value="{{ old('target_identity_id', $record?->identity_id) }}" @readonly($record !== null)>

        <label for="category">{{ __('support.common.category') }}</label>
        <select id="category" name="category" required>
            @foreach ($categories as $category)
                <option value="{{ $category }}" @selected(old('category', $record?->category) === $category)>{{ __('support.category.'.$category) }}</option>
            @endforeach
        </select>

        <label for="status">{{ __('support.common.status') }}</label>
        <select id="status" name="status" required>
            @foreach ($statuses as $status)
                <option value="{{ $status }}" @selected(old('status', $record?->status ?? 'active') === $status)>{{ __('support.enforcement_status.'.$status) }}</option>
            @endforeach
        </select>

        <label for="public_reason">{{ __('support.enforcement.public_reason') }}</label>
        <textarea id="public_reason" name="public_reason" rows="8" maxlength="4000" required>{{ old('public_reason', $record?->public_reason) }}</textarea>

        <label for="moderator_notes">{{ __('support.enforcement.moderator_notes') }}</label>
        <textarea id="moderator_notes" name="moderator_notes" rows="8" maxlength="8000">{{ old('moderator_notes', $record?->moderator_notes) }}</textarea>

        <label for="effective_at">{{ __('support.enforcement.effective_at') }}</label>
        <input id="effective_at" name="effective_at" type="datetime-local" required value="{{ old('effective_at', $record?->effective_at?->utc()->format('Y-m-d\TH:i') ?? now()->utc()->format('Y-m-d\TH:i')) }}">

        <label for="expires_at">{{ __('support.enforcement.expires_at') }}</label>
        <input id="expires_at" name="expires_at" type="datetime-local" value="{{ old('expires_at', $record?->expires_at?->utc()->format('Y-m-d\TH:i')) }}">

        <div class="action-row">
            <button class="button" type="submit">{{ __('support.common.save') }}</button>
            <a class="button button-secondary" href="{{ route('admin.moderation.enforcement.index', ['locale' => app()->getLocale()]) }}">{{ __('support.common.back') }}</a>
        </div>
    </form>
@endsection
