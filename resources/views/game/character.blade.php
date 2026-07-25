@extends('game.layout')

@section('title', $character->name)

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <h1>{{ $character->name }}</h1>

    <div class="card">
        <dl>
            <dt>{{ __('public.game.level') }}</dt>
            <dd>{{ $localeFormatter->number($character->level) }}</dd>
            <dt>{{ __('public.game.vocation_id') }}</dt>
            <dd>{{ $localeFormatter->number($character->vocation) }}</dd>
        </dl>
    </div>
@endsection
