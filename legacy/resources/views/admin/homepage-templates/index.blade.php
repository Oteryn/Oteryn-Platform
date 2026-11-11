@extends('admin.layout')

@section('title', __('homepage_templates.admin.title'))

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('homepage_templates.admin.eyebrow') }}</p>
        <h1>{{ __('homepage_templates.admin.heading') }}</h1>
        <p class="muted">{{ __('homepage_templates.admin.intro') }}</p>
    </div>

    @if (session('error'))
        <p class="notice" role="alert">{{ session('error') }}</p>
    @endif

    @if ($errors->any())
        <div class="notice" role="alert">
            <ul>
                @foreach ($errors->all() as $error)
                    <li>{{ $error }}</li>
                @endforeach
            </ul>
        </div>
    @endif

    @if ($snapshot->drifted)
        <p class="notice" role="alert">{{ __('homepage_templates.admin.drift_warning') }}</p>
    @endif

    <div class="card">
        <h2>{{ __('homepage_templates.admin.current') }}</h2>
        <p>
            <strong>{{ __($templates[$snapshot->activeKey]['label']) }}</strong>
            · {{ __('homepage_templates.admin.version', ['version' => $snapshot->version]) }}
        </p>
        @if ($snapshot->canRollback())
            <form method="POST" action="{{ route('admin.homepage-templates.rollback') }}">
                @csrf
                <input type="hidden" name="version" value="{{ $snapshot->version }}">
                <button type="submit" class="button button-secondary">
                    {{ __('homepage_templates.admin.rollback', ['template' => __($templates[$snapshot->previousKey]['label'])]) }}
                </button>
            </form>
        @else
            <p class="muted">{{ __('homepage_templates.admin.rollback_help') }}</p>
        @endif
    </div>

    <div class="card-grid">
        @foreach ($templates as $key => $template)
            <article class="card">
                <h2>{{ __($template['label']) }}</h2>
                <p class="muted">{{ __($template['description']) }}</p>

                @if ($snapshot->activeKey === $key)
                    <p><strong>{{ __('homepage_templates.admin.active') }}</strong></p>
                @endif

                <div class="action-row">
                    <a class="button button-secondary" href="{{ route('admin.homepage-templates.preview', ['template' => $key]) }}" target="_blank" rel="noopener">
                        {{ __('homepage_templates.admin.preview') }}
                    </a>

                    @if ($snapshot->activeKey !== $key)
                        <form method="POST" action="{{ route('admin.homepage-templates.activate') }}">
                            @csrf
                            @method('PUT')
                            <input type="hidden" name="template" value="{{ $key }}">
                            <input type="hidden" name="version" value="{{ $snapshot->version }}">
                            <button type="submit">{{ __('homepage_templates.admin.activate') }}</button>
                        </form>
                    @endif
                </div>
            </article>
        @endforeach
    </div>
@endsection
