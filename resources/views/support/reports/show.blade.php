@extends('identity.layout')

@section('title', __('support.title.report_detail'))

@section('content')
    @include('support.partials.navigation')
    @include('identity.partials.locale-switcher', [
        'localeRoute' => 'support.reports.show',
        'localeParameters' => ['playerReport' => $report],
    ])

    <p class="eyebrow">{{ $report->public_id }}</p>
    <h1>{{ __('support.title.report_detail') }}</h1>

    <dl class="detail-grid">
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

    <h2>{{ __('support.common.outcome') }}</h2>
    <p class="pre-wrap">{{ $report->public_outcome ?: __('support.common.not_available') }}</p>

    <a class="button button-secondary" href="{{ route('support.reports.index', ['locale' => app()->getLocale()]) }}">{{ __('support.common.back') }}</a>
@endsection
