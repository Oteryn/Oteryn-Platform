@extends('identity.layout')

@section('title', __('support.title.report_create'))

@section('content')
    @include('support.partials.navigation')
    @include('identity.partials.locale-switcher', ['localeRoute' => 'support.reports.create'])

    <p class="eyebrow">{{ __('support.nav.support_center') }}</p>
    <h1>{{ __('support.title.report_create') }}</h1>
    <p class="supporting-copy">{{ __('support.reports.privacy_notice') }}</p>
    <p class="supporting-copy">{{ __('support.common.attachments_disabled') }}</p>

    <form method="POST" action="{{ route('support.reports.store', ['locale' => app()->getLocale()]) }}" class="form-stack">
        @csrf
        <input type="hidden" name="request_key" value="{{ old('request_key', $requestKey) }}">

        <label for="report_type">{{ __('support.reports.type') }}</label>
        <select id="report_type" name="report_type" required>
            @foreach ($types as $type)
                <option value="{{ $type }}" @selected(old('report_type') === $type)>{{ __('support.type.'.$type) }}</option>
            @endforeach
        </select>

        <label for="category">{{ __('support.common.category') }}</label>
        <select id="category" name="category" required>
            @foreach (array_values(array_unique(array_merge(...array_values($categories)))) as $category)
                <option value="{{ $category }}" @selected(old('category') === $category)>{{ __('support.category.'.$category) }}</option>
            @endforeach
        </select>

        <label for="target_reference">{{ __('support.common.target') }}</label>
        <input id="target_reference" name="target_reference" type="text" maxlength="160" required value="{{ old('target_reference') }}">

        <label for="evidence_summary">{{ __('support.reports.evidence') }}</label>
        <textarea id="evidence_summary" name="evidence_summary" rows="9" maxlength="4000">{{ old('evidence_summary') }}</textarea>

        <div class="action-row">
            <button class="button" type="submit">{{ __('support.reports.submit') }}</button>
            <a class="button button-secondary" href="{{ route('support.reports.index', ['locale' => app()->getLocale()]) }}">{{ __('support.common.back') }}</a>
        </div>
    </form>
@endsection
