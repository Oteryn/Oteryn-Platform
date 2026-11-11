@extends('admin.layout')

@section('title', __('support.title.report_detail'))

@section('content')
    <p class="eyebrow">{{ $report->public_id }}</p>
    <h1>{{ __('support.title.report_detail') }}</h1>

    <dl class="detail-grid">
        <div><dt>{{ __('support.reports.reporter') }}</dt><dd>#{{ $report->reporter_identity_id }} · {{ $report->reporter?->email }}</dd></div>
        <div><dt>{{ __('support.reports.type') }}</dt><dd>{{ __('support.type.'.$report->report_type) }}</dd></div>
        <div><dt>{{ __('support.common.category') }}</dt><dd>{{ __('support.category.'.$report->category) }}</dd></div>
        <div><dt>{{ __('support.common.target') }}</dt><dd>{{ $report->target_reference }}</dd></div>
        <div><dt>{{ __('support.common.status') }}</dt><dd>{{ __('support.report_status.'.$report->status) }}</dd></div>
        <div><dt>{{ __('support.common.created') }}</dt><dd>{{ $report->created_at->toDateTimeString() }}</dd></div>
    </dl>

    @if ($report->evidence_summary)
        <h2>{{ __('support.reports.evidence') }}</h2>
        <p class="pre-wrap">{{ $report->evidence_summary }}</p>
    @endif

    <form method="POST" action="{{ route('admin.moderation.reports.update', ['playerReport' => $report, 'locale' => app()->getLocale()]) }}" class="form-stack">
        @csrf
        @method('PUT')
        <input type="hidden" name="lock_version" value="{{ $report->lock_version }}">

        <label for="status">{{ __('support.common.status') }}</label>
        <select id="status" name="status" required>
            @foreach ($statuses as $status)
                <option value="{{ $status }}" @selected(old('status', $report->status) === $status)>{{ __('support.report_status.'.$status) }}</option>
            @endforeach
        </select>

        <label for="public_outcome">{{ __('support.reports.public_outcome') }}</label>
        <textarea id="public_outcome" name="public_outcome" rows="6" maxlength="4000">{{ old('public_outcome', $report->public_outcome) }}</textarea>

        <label for="moderator_notes">{{ __('support.reports.moderator_notes') }}</label>
        <textarea id="moderator_notes" name="moderator_notes" rows="8" maxlength="8000">{{ old('moderator_notes', $report->moderator_notes) }}</textarea>

        <button class="button" type="submit">{{ __('support.common.save') }}</button>
    </form>

    <a class="button button-secondary" href="{{ route('admin.moderation.reports.index', ['locale' => app()->getLocale()]) }}">{{ __('support.common.back') }}</a>
@endsection
