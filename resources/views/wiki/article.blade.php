@extends('game.layout')

@section('title', $article->title)
@section('description', $article->summary)
@section('og-type', 'article')
@section('page-class', 'wiki-page wiki-article-page')

@push('head')
    <meta name="description" content="{{ \Illuminate\Support\Str::limit($article->summary, 155) }}">
    <script src="{{ asset('js/media-fallbacks.js') }}" defer></script>
@endpush
@push('styles')
    <link rel="stylesheet" href="{{ asset('css/wiki.css') }}">
@endpush

@section('content')
    @include('wiki.partials.breadcrumbs', ['breadcrumbs' => $article->breadcrumbs])

    <header class="page-header wiki-article-header">
        <p class="eyebrow">{{ __('public.wiki.article') }}</p>
        <h1>{{ $article->title }}</h1>
        <p class="wiki-summary">{{ $article->summary }}</p>
        <p class="muted">{{ __('public.wiki.published', ['date' => $article->publishedAt->toDateString()]) }}</p>
        @if ($article->categories !== [])
            <ul class="wiki-category-pills" aria-label="{{ __('public.wiki.categories') }}">
                @foreach ($article->categories as $category)
                    <li><a href="{{ route('wiki.category', ['slug' => $category->slug]) }}">{{ $category->name }}</a></li>
                @endforeach
            </ul>
        @endif
    </header>

    <div class="wiki-article-layout">
        @if ($rendered->tableOfContents !== [])
            <aside class="wiki-toc" aria-labelledby="wiki-toc-title">
                <details open>
                    <summary id="wiki-toc-title">{{ __('public.wiki.contents') }}</summary>
                    <ol>
                        @foreach ($rendered->tableOfContents as $item)
                            <li class="wiki-toc-level-{{ $item->level }}">
                                <a href="#{{ $item->id }}">{{ $item->title }}</a>
                            </li>
                        @endforeach
                    </ol>
                </details>
            </aside>
        @endif

        <article class="wiki-markdown">
            {!! $rendered->html !!}
        </article>
    </div>

    @if ($article->relatedArticles !== [])
        <section aria-labelledby="wiki-related">
            <div class="section-heading">
                <h2 id="wiki-related">{{ __('public.wiki.related') }}</h2>
            </div>
            <div class="card-grid">
                @foreach ($article->relatedArticles as $articleCard)
                    @include('wiki.partials.article-card', ['articleCard' => $articleCard])
                @endforeach
            </div>
        </section>
    @endif
@endsection
