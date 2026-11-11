@extends('admin.layout')

@section('title', __('support.title.admin_reports'))

@section('content')
    <p class="eyebrow">{{ __('support.nav.admin_reports') }}</p>
    <h1>{{ __('support.title.admin_reports') }}</h1>

    <form method="GET" action="{{ route('admin.moderation.reports.index') }}" class="filter-row">
        <input type="hidden" name="locale" value="{{ app()->getLocale() }}">
        <label for="status">{{ __('support.common.status') }}</label>
        <select id="status" name="status">
            <option value="">{{ __('support.common.all_statuses') }}</option>
            @foreach ($statuses as $status)
                <option value="{{ $status }}" @selected($selectedStatus === $status)>{{ __('support.report_status.'.$status) }}</option>
            @endforeach
        </select>
        <button class="button button-secondary" type="submit">{{ __('support.common.filter') }}</button>
    </form>

    @if ($reports->isEmpty())
        <div class="empty-state"><p>{{ __('support.reports.queue_empty') }}</p></div>
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
                        <td><a href="{{ route('admin.moderation.reports.show', ['playerReport' => $report, 'locale' => app()->getLocale()]) }}">{{ __('support.common.view') }}</a></td>
                    </tr>
                @endforeach
                </tbody>
            </table>
        </div>
        {{ $reports->links() }}
    @endif
@endsection
