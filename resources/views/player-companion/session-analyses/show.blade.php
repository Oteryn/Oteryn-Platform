@extends('game.layout')

@section('title', $analysis->label ?: __('player_companion.session_analyzer'))
@section('description', __('player_companion.privacy_note'))
@section('page-class', 'player-companion-page')

@section('content')
    <div class="page-header">
        <div>
            <p class="eyebrow">{{ __('player_companion.deterministic') }}</p>
            <h1>{{ $analysis->label ?: __('player_companion.session_analyzer') }}</h1>
            <p class="muted">{{ __('player_companion.privacy_note') }}</p>
        </div>
        <div class="action-row">
            <a class="button button-secondary" href="{{ route('player-companion.session-analyses.index') }}">{{ __('player_companion.back') }}</a>
            <form method="POST" action="{{ route('player-companion.session-analyses.destroy', $analysis->id) }}" onsubmit="return confirm(@js(__('player_companion.confirm_delete')))">
                @csrf
                @method('DELETE')
                <button class="button button-secondary" type="submit">{{ __('player_companion.delete') }}</button>
            </form>
        </div>
    </div>

    @if (session('status'))
        <div class="notice notice-success" role="status">{{ session('status') }}</div>
    @endif

    <section class="content-card">
        <div class="stats-grid" data-testid="session-analysis-metrics">
            @php
                $metrics = [
                    __('player_companion.session_duration') => sprintf('%02d:%02d', intdiv($analysis->session_seconds, 3600), intdiv($analysis->session_seconds % 3600, 60)),
                    __('player_companion.experience_gain') => $analysis->experience_gain,
                    __('player_companion.experience_per_hour') => $analysis->experience_per_hour,
                    __('player_companion.loot') => $analysis->loot_value,
                    __('player_companion.supplies') => $analysis->supplies_value,
                    __('player_companion.balance') => $analysis->balance_value,
                    __('player_companion.profit_per_hour') => $analysis->profit_per_hour,
                    __('player_companion.damage') => $analysis->damage,
                    __('player_companion.healing') => $analysis->healing,
                ];
            @endphp
            @foreach ($metrics as $label => $value)
                <div class="stat-card">
                    <span class="muted">{{ $label }}</span>
                    <strong>{{ is_int($value) ? number_format($value) : ($value ?? '—') }}</strong>
                </div>
            @endforeach
        </div>
    </section>

    <section class="content-card" aria-labelledby="participants-title">
        <div class="section-heading">
            <h2 id="participants-title">{{ __('player_companion.participants') }}</h2>
        </div>
        @if ($analysis->participant_count === 0)
            <div class="empty-state"><strong>—</strong></div>
        @else
            <div class="table-region">
                <table>
                    <thead><tr><th>{{ __('player_companion.participants') }}</th><th>{{ __('player_companion.loot') }}</th><th>{{ __('player_companion.supplies') }}</th><th>{{ __('player_companion.balance') }}</th><th>{{ __('player_companion.damage') }}</th><th>{{ __('player_companion.healing') }}</th></tr></thead>
                    <tbody>
                        @foreach ($analysis->participants as $participant)
                            <tr>
                                <td>{{ $participant['name'] }}</td>
                                <td>{{ $participant['loot_value'] === null ? '—' : number_format($participant['loot_value']) }}</td>
                                <td>{{ $participant['supplies_value'] === null ? '—' : number_format($participant['supplies_value']) }}</td>
                                <td>{{ $participant['balance_value'] === null ? '—' : number_format($participant['balance_value']) }}</td>
                                <td>{{ $participant['damage'] === null ? '—' : number_format($participant['damage']) }}</td>
                                <td>{{ $participant['healing'] === null ? '—' : number_format($participant['healing']) }}</td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>

    <section class="content-card" aria-labelledby="settlements-title">
        <div class="section-heading">
            <h2 id="settlements-title">{{ __('player_companion.settlements') }}</h2>
            <p class="muted">{{ __('player_companion.advisory_split') }}</p>
        </div>
        @if ($analysis->settlements === [])
            <div class="empty-state"><strong>{{ __('player_companion.settlements_empty') }}</strong></div>
        @else
            <div class="table-region">
                <table data-testid="session-analysis-settlements">
                    <thead><tr><th>{{ __('player_companion.from') }}</th><th>{{ __('player_companion.to') }}</th><th>{{ __('player_companion.amount') }}</th></tr></thead>
                    <tbody>
                        @foreach ($analysis->settlements as $settlement)
                            <tr><td>{{ $settlement['from'] }}</td><td>{{ $settlement['to'] }}</td><td>{{ number_format($settlement['amount']) }}</td></tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>

    <section class="content-card">
        <dl>
            <dt>{{ __('player_companion.source_format') }}</dt><dd>{{ $analysis->source_format }}</dd>
            <dt>{{ __('player_companion.parser_version') }}</dt><dd>{{ $analysis->parser_version }}</dd>
            <dt>{{ __('player_companion.formula_version') }}</dt><dd>{{ $analysis->formula_version }}</dd>
        </dl>
    </section>
@endsection
