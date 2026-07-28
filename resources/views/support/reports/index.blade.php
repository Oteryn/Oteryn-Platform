@extends('identity.layout')

@section('title', __('support.title.reports'))

@section('content')
    @include('support.partials.navigation')
    @include('identity.partials.locale-switcher', ['localeRoute' => 'support.reports.index'])

    <div class="section-heading">
        <div><p class="eyebrow">{{ __('support.nav.support_center') }}</p><h1>{{ __('support.title.reports') }}</h1></div>
        <a class="button" href="{{ route('support.reports.create', ['locale' => app()->getLocale()]) }}">{{ __('support.reports.submit') }}</a>
    </div>

    @if ($reports->isEmpty())
        <div class="empty-state"><p>{{ __('support.reports.empty') }}</p></div>
    @else
        <div class="table-wrap">
            <table>
                <thead><tr><th>{{ __('support.reports.type') }}</th><th>{{ __('support.common.target') }}</th><th>{{ __('support.common.status') }}</th><th>{{ __('support.common.created') }}</th><th></th></tr></thead>
                <tbody>
                @foreach ($reports as $report)
                    <tr>
                        <td>{{ __('support.type.'.$report->report_type) }}</td>
                        <td>{{ $report->target_reference }}</td>
                        <td>{{ __('support.report_status.'.$report->status) }}</td>
                        <td>{{ $report->created_at->toDateTimeString() }}</td>
                        <td><a href="{{ route('support.reports.show', ['playerReport' => $report, 'locale' => app()->getLocale()]) }}">{{ __('support.common.view') }}</a></td>
                    </tr>
                @endforeach
                </tbody>
            </table>
        </div>
        {{ $reports->links() }}
    @endif
@endsection
