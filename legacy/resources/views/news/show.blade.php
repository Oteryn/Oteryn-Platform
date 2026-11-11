@extends('game.layout')

@section('title', $post->title)
@section('description', \Illuminate\Support\Str::limit(strip_tags($post->body), 160, ''))
@section('og-type', 'article')
@section('portal-family', 'chronicles')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <a class="reading-back" href="{{ route('news.index') }}"><span aria-hidden="true">←</span> {{ __('portal.common.back_news') }}</a>
    <article class="reading-article">
        <header class="reading-masthead">
        <p class="eyebrow">{{ __('portal.home.chronicles') }}</p>
        <h1>{{ $post->title }}</h1>
        <p class="muted">{{ __('public.news.published', ['date' => $post->published_at ? $localeFormatter->dateTime($post->published_at) : '']) }}</p>
        </header>
        <div class="card reading-body">
            <p class="prose-text">{{ $post->body }}</p>
        </div>
    </article>
@endsection
