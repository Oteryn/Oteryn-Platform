@extends('identity.layout')

@section('title', __('portal.create.request_title'))
@section('error-title', __('portal.create.error'))

@section('portal-family', 'character-create')

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal.create.eyebrow') }}</p>
        <h1>{{ __('portal.create.request_heading') }}</h1>
        <p class="muted" role="status" data-outcome="{{ $outcome->state }}">{{ $outcome->message }}</p>
    </div>
    <div class="identity-links">
        <a href="{{ route('account.characters.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.create.request_again') }}</a>
        <a href="{{ route('account.overview', ['locale' => app()->getLocale()]) }}">{{ __('portal.account.characters') }}</a>
    </div>
@endsection
