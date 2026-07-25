@extends('game.layout')

@section('title', $event['title'])

@section('content')
    <article>
        <div class="page-header">
            <p class="eyebrow">{{ ucfirst($event['status']) }} event</p>
            <h1>{{ $event['title'] }}</h1>
            <p class="muted">
                {{ $event['starts_at']->format('Y-m-d H:i') }}
                –
                {{ $event['ends_at']->format('Y-m-d H:i') }} UTC
            </p>
            <p>{{ $event['summary'] }}</p>
        </div>

        <div class="card content-copy">
            @foreach (preg_split('/\R{2,}/', $event['body']) ?: [$event['body']] as $paragraph)
                <p>{{ $paragraph }}</p>
            @endforeach
        </div>

        @if ($event['news_slug'] !== null && $event['news_title'] !== null)
            <div class="card">
                <p class="eyebrow">Related update</p>
                <a href="{{ route('news.show', ['slug' => $event['news_slug']]) }}">{{ $event['news_title'] }}</a>
            </div>
        @endif

        <p><a href="{{ route('events.index') }}">Back to events</a></p>
    </article>
@endsection
