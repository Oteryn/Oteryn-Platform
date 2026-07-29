@extends('admin.layout')

@section('title', __('support.title.admin_enforcement'))

@section('content')
    <div class="section-heading">
        <div><p class="eyebrow">{{ __('support.nav.admin_enforcement') }}</p><h1>{{ __('support.title.admin_enforcement') }}</h1></div>
        <a class="button" href="{{ route('admin.moderation.enforcement.create', ['locale' => app()->getLocale()]) }}">{{ __('support.enforcement.create') }}</a>
    </div>

    <form method="GET" action="{{ route('admin.moderation.enforcement.index') }}" class="filter-row">
        <input type="hidden" name="locale" value="{{ app()->getLocale() }}">
        <label for="status">{{ __('support.common.status') }}</label>
        <select id="status" name="status">
            <option value="">{{ __('support.common.all_statuses') }}</option>
            @foreach ($statuses as $status)
                <option value="{{ $status }}" @selected($selectedStatus === $status)>{{ __('support.enforcement_status.'.$status) }}</option>
            @endforeach
        </select>
        <button class="button button-secondary" type="submit">{{ __('support.common.filter') }}</button>
    </form>

    @if ($records->isEmpty())
        <div class="empty-state"><p>{{ __('support.enforcement.empty') }}</p></div>
    @else
        <div class="table-wrap">
            <table>
                <thead><tr><th>{{ __('support.common.identity_id') }}</th><th>{{ __('support.common.category') }}</th><th>{{ __('support.common.status') }}</th><th>{{ __('support.enforcement.effective_at') }}</th><th>{{ __('support.enforcement.appeal_status') }}</th><th></th></tr></thead>
                <tbody>
                @foreach ($records as $record)
                    <tr>
                        <td>#{{ $record->identity_id }}</td>
                        <td>{{ __('support.category.'.$record->category) }}</td>
                        <td>{{ __('support.enforcement_status.'.$record->status) }}</td>
                        <td>{{ $record->effective_at->toDateTimeString() }}</td>
                        <td>{{ __('support.appeal_status.'.$record->appeal_status) }}</td>
                        <td><a href="{{ route('admin.moderation.enforcement.show', ['enforcementRecord' => $record, 'locale' => app()->getLocale()]) }}">{{ __('support.common.view') }}</a></td>
                    </tr>
                @endforeach
                </tbody>
            </table>
        </div>
        {{ $records->links() }}
    @endif
@endsection
