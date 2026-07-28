@extends('game.layout')

@section('title', __('game_catalog.unavailable'))
@section('description', __('game_catalog.unavailable_help'))
@section('robots', 'noindex,nofollow')
@section('page-class', 'game-catalog-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <header class="page-header">
        <p class="eyebrow">{{ __('game_catalog.eyebrow') }}</p>
        <h1>{{ __('game_catalog.unavailable') }}</h1>
    </header>
    <div class="empty-state" role="alert">
        <strong>{{ __('game_catalog.unavailable') }}</strong>
        <p>{{ __('game_catalog.unavailable_help') }}</p>
    </div>
@endsection
