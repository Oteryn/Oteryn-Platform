@extends('identity.layout')

@section('title', __('portal.create.request_title'))
@section('error-title', __('portal.create.error'))

@section('portal-family', 'character-create')

@if ($outcome->state === 'pending')
    @push('head')
        <script src="{{ asset('js/character-request-status.js') }}?v={{ substr(hash_file('sha256', public_path('js/character-request-status.js')), 0, 12) }}" defer></script>
    @endpush
@endif

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal.create.eyebrow') }}</p>
        <h1>{{ __('portal.create.request_heading') }}</h1>
        <p class="muted" role="status" data-outcome="{{ $outcome->state }}">{{ $outcome->message }}</p>
    </div>
    <div class="identity-links">
        @if ($outcome->state === 'pending')
            <a class="button" data-character-request-check href="{{ route('account.characters.requests.show', ['operation' => request()->route()->parameter('operation'), 'locale' => app()->getLocale()]) }}">{{ __('portal.create.request_check') }}</a>
        @else
            <a href="{{ route('account.characters.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.create.request_again') }}</a>
        @endif
        <a href="{{ route('account.overview', ['locale' => app()->getLocale()]) }}">{{ __('portal.account.characters') }}</a>
    </div>
@endsection
