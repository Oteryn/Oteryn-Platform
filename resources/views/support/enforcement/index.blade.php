@extends('identity.layout')

@section('title', __('support.title.enforcement'))

@section('content')
    @include('support.partials.navigation')
    @include('identity.partials.locale-switcher', ['localeRoute' => 'support.enforcement.index'])

    <p class="eyebrow">{{ __('support.nav.support_center') }}</p>
    <h1>{{ __('support.title.enforcement') }}</h1>
    <p class="supporting-copy">{{ __('support.enforcement.platform_only') }}</p>

    @if ($records->isEmpty())
        <div class="empty-state"><p>{{ __('support.enforcement.empty') }}</p></div>
    @else
        <div class="table-wrap">
            <table>
                <thead><tr><th>{{ __('support.common.category') }}</th><th>{{ __('support.common.status') }}</th><th>{{ __('support.enforcement.effective_at') }}</th><th>{{ __('support.enforcement.appeal_status') }}</th><th></th></tr></thead>
                <tbody>
                @foreach ($records as $record)
                    <tr>
                        <td>{{ __('support.category.'.$record->category) }}</td>
                        <td>{{ __('support.enforcement_status.'.$record->status) }}</td>
                        <td>{{ $record->effective_at->toDateTimeString() }}</td>
                        <td>{{ __('support.appeal_status.'.$record->appeal_status) }}</td>
                        <td><a href="{{ route('support.enforcement.show', ['enforcementRecord' => $record, 'locale' => app()->getLocale()]) }}">{{ __('support.common.view') }}</a></td>
                    </tr>
                @endforeach
                </tbody>
            </table>
        </div>
        {{ $records->links() }}
    @endif
@endsection
