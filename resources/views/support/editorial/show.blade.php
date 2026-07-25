@extends('game.layout')

@section('title', $result->page?->title ?? $key->label())

@section('content')
    @if ($result->state === \App\Cms\Editorial\EditorialPageState::Published)
        @php($page = $result->page)
        <article>
            <p class="eyebrow">{{ $key->isLegal() ? 'Legal' : ($key->isSupportGuidance() ? 'Support' : 'Learn') }}</p>
            <h1>{{ $page->title }}</h1>

            @if ($key->isLegal() && $page->legal_version !== null && $page->legal_effective_date !== null)
                <p class="muted">
                    Version {{ $page->legal_version }}
                    · Effective {{ $page->legal_effective_date->format('Y-m-d') }}
                </p>
            @endif

            <div class="card">
                <p class="prose-text">{{ $page->body }}</p>
            </div>

            @if ($supportLinks !== [])
                <section aria-labelledby="approved-support-links">
                    <h2 id="approved-support-links">Approved support channels</h2>
                    <div class="card-grid">
                        @foreach ($supportLinks as $link)
                            <article class="card">
                                <h3>{{ $link['label'] }}</h3>
                                @if ($link['detail'] !== null)
                                    <p class="muted">{{ $link['detail'] }}</p>
                                @endif
                                <a class="button button-secondary"
                                   href="{{ $link['href'] }}"
                                   @if ($link['external']) target="_blank" rel="noopener noreferrer" @endif>
                                    Open
                                </a>
                            </article>
                        @endforeach
                    </div>
                </section>
            @endif

            @if ($key === \App\Cms\Editorial\EditorialPageKey::ReportABug)
                <div class="notice">
                    Do not submit passwords, MFA secrets, recovery codes, payment details or other unnecessary personal data.
                    This page provides guidance only; Oteryn Platform does not store a support ticket submission here.
                </div>
            @endif
        </article>
    @else
        <article>
            <p class="eyebrow">Editorial content</p>
            <h1>{{ $key->label() }}</h1>

            <div class="empty-state">
                @if ($result->state === \App\Cms\Editorial\EditorialPageState::Missing)
                    <strong>This editorial page has not been configured.</strong>
                    <p>The requested route is stable, but no managed page currently exists for it.</p>
                @else
                    <strong>This editorial page is not currently published.</strong>
                    <p>Draft and future-scheduled content cannot be accessed publicly.</p>
                @endif
            </div>
        </article>
    @endif
@endsection
