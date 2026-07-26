@extends('game.layout')

@section('title', __('public.wiki.unavailable'))
@section('robots', 'noindex,nofollow')
@section('page-class', 'wiki-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/wiki.css') }}">
@endpush

@section('content')
    <div class="empty-state" role="alert">
        <strong>{{ __('public.wiki.unavailable') }}</strong>
        <p>{{ __('public.wiki.unavailable_help') }}</p>
        <a class="button" href="{{ route('localized.home', ['locale' => app()->getLocale()]) }}">{{ __('public.errors.back_home') }}</a>
    </div>
@endsection
