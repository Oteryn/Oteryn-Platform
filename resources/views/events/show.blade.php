@extends('game.layout')

@section('title', $event['title'])

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <article>
        <div class="page-header">
            <p class="eyebrow">{{ ucfirst($event['status']) }} {{ __('public.events.event') }}</p>
            <h1>{{ $event['title'] }}</h1>
            <p class="muted">
                {{ $localeFormatter->dateTime($event['starts_at']) }}
                –
                {{ $localeFormatter->dateTime($event['ends_at']) }} UTC
            </p>
            <p>{{ $event['summary'] }}</p>
        </div>

        <div class="card content-copy">
            @foreach (preg_split('/\R{2,}/', $event['body']) ?: [$event['body']] as $paragraph)
                <p>{{ $paragraph }}</p>
            @endforeach
        </div>

        @if (app()->getLocale() === 'en' && $event['news_slug'] !== null && $event['news_title'] !== null)
            <div class="card">
                <p class="eyebrow">{{ __('public.events.related_update') }}</p>
                <a href="{{ route('news.show', ['slug' => $event['news_slug']]) }}">{{ $event['news_title'] }}</a>
            </div>
        @endif

        <p><a href="{{ route('events.index') }}">{{ __('public.events.back') }}</a></p>
    </article>
@endsection
