@extends('game.layout')

@section('title', __('player_companion.session_analyzer'))
@section('description', __('player_companion.session_analyzer_intro'))
@section('page-class', 'player-companion-page')

@section('content')
    <div class="page-header">
        <div>
            <p class="eyebrow">Player Companion · SessionAnalysis</p>
            <h1>{{ __('player_companion.session_analyzer') }}</h1>
            <p class="muted">{{ __('player_companion.session_analyzer_intro') }}</p>
        </div>
    </div>

    @if (session('status'))
        <div class="notice notice-success" role="status">{{ session('status') }}</div>
    @endif

    @if ($errors->any())
        <div class="notice notice-danger" role="alert">
            <strong>{{ $errors->first() }}</strong>
        </div>
    @endif

    <section class="content-card" aria-labelledby="session-analyzer-form-title">
        <div class="section-heading">
            <p class="eyebrow">{{ __('player_companion.deterministic') }}</p>
            <h2 id="session-analyzer-form-title">{{ __('player_companion.session_analyzer') }}</h2>
        </div>

        <form method="POST" action="{{ route('player-companion.session-analyses.store') }}">
            @csrf
            <div class="form-grid">
                <label>
                    <span>{{ __('player_companion.label') }}</span>
                    <input type="text" name="label" maxlength="80" value="{{ old('label') }}" placeholder="{{ __('player_companion.label_hint') }}">
                </label>
                <label class="form-field-wide">
                    <span>{{ __('player_companion.session_log') }}</span>
                    <textarea name="session_log" rows="16" maxlength="65535" required aria-describedby="session-log-help"></textarea>
                    <small id="session-log-help" class="muted">{{ __('player_companion.session_log_hint') }}</small>
                </label>
            </div>
            <p class="muted">{{ __('player_companion.privacy_note') }}</p>
            <button class="button" type="submit">{{ __('player_companion.analyze_save') }}</button>
        </form>
    </section>

    <section class="content-card" aria-labelledby="session-history-title">
        <div class="section-heading">
            <p class="eyebrow">{{ __('player_companion.history') }}</p>
            <h2 id="session-history-title">{{ __('player_companion.history') }}</h2>
        </div>

        @if ($analyses->isEmpty())
            <div class="empty-state" data-testid="session-analysis-empty">
                <strong>{{ __('player_companion.history_empty') }}</strong>
            </div>
        @else
            <div class="table-region">
                <table>
                    <thead>
                        <tr>
                            <th>{{ __('player_companion.label') }}</th>
                            <th>{{ __('player_companion.session_duration') }}</th>
                            <th>{{ __('player_companion.balance') }}</th>
                            <th>{{ __('player_companion.created_at') }}</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        @foreach ($analyses as $analysis)
                            <tr>
                                <td>{{ $analysis->label ?: __('player_companion.session_analyzer') }}</td>
                                <td>{{ sprintf('%02d:%02d', intdiv($analysis->session_seconds, 3600), intdiv($analysis->session_seconds % 3600, 60)) }}</td>
                                <td>{{ $analysis->balance_value === null ? '—' : number_format($analysis->balance_value) }}</td>
                                <td>{{ $analysis->created_at?->format('Y-m-d H:i') }}</td>
                                <td><a href="{{ route('player-companion.session-analyses.show', $analysis->id) }}">{{ __('player_companion.open') }}</a></td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>
@endsection
