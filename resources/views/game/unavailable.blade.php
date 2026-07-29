@extends('game.layout')

@section('title', $title)
@section('robots', 'noindex,nofollow')
@section('page-class', 'community-page')

@section('content')
    <section class="card community-unavailable" role="status" aria-labelledby="community-unavailable-heading">
        <p class="eyebrow">{{ __('community.unavailable.eyebrow') }}</p>
        <h1 id="community-unavailable-heading">{{ __('community.unavailable.title') }}</h1>
        <p>{{ __('community.unavailable.description') }}</p>
        <div class="action-row">
            <a class="button" href="{{ request()->fullUrl() }}">{{ __('community.unavailable.retry') }}</a>
        </div>
    </section>
@endsection
