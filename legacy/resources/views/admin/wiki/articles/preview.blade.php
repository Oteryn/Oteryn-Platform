@extends('admin.layout')

@section('title', 'Preview · '.$translation->title)

@section('content')
    <div class="wiki-preview">
        <div class="page-header">
            <p class="eyebrow">Wiki · Signed draft preview · {{ strtoupper($translation->locale) }}</p>
            <h1>{{ $translation->title }}</h1>
            <p class="muted">This short-lived URL requires authentication, confirmed MFA, an exact Wiki article permission and a valid signature.</p>
        </div>

        <div class="action-row">
            <a class="button button-secondary" href="{{ route('admin.wiki.articles.edit', $article) }}">Return to editor</a>
        </div>

        <div class="wiki-preview-layout">
            <article class="card wiki-preview-content">
                @if ($translation->summary !== '')
                    <p class="lead">{{ $translation->summary }}</p>
                @endif
                <div class="wiki-article-body">
                    {!! $rendered->html !!}
                </div>
            </article>

            @if ($rendered->tableOfContents !== [])
                <aside class="card wiki-preview-toc" aria-label="Article table of contents">
                    <h2>On this page</h2>
                    <ol>
                        @foreach ($rendered->tableOfContents as $item)
                            <li class="wiki-toc-level-{{ $item->level }}">
                                <a href="#{{ $item->id }}">{{ $item->title }}</a>
                            </li>
                        @endforeach
                    </ol>
                </aside>
            @endif
        </div>
    </div>
@endsection
