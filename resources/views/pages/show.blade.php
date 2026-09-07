@extends('game.layout')

@section('title', $page->title)
@section('description', \Illuminate\Support\Str::limit(strip_tags($page->body), 160, ''))

@section('portal-family', 'editorial')

@section('content')
    <article class="reading-article">
        <h1>{{ $page->title }}</h1>

        <div class="card reading-body">
            <p class="prose-text">{{ $page->body }}</p>
        </div>
    </article>
@endsection
