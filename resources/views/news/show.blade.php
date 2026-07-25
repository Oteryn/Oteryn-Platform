@extends('game.layout')

@section('title', $post->title)

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <article>
        <h1>{{ $post->title }}</h1>
        <p class="muted">{{ __('public.news.published', ['date' => $post->published_at ? $localeFormatter->dateTime($post->published_at) : '']) }}</p>

        <div class="card">
            <p class="prose-text">{{ $post->body }}</p>
        </div>
    </article>
@endsection
