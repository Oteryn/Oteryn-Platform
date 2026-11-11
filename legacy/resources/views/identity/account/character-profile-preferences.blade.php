@extends('identity.layout')

@section('title', __('character_profiles.title'))

@section('portal-family', 'profile-preferences')

@section('content')
    <header class="page-header">
        <p class="eyebrow">{{ __('character_profiles.eyebrow') }}</p>
        <h1>{{ $character->name }}</h1>
        <p class="muted">{{ __('character_profiles.description') }}</p>
    </header>

    <form method="POST" action="{{ route('account.characters.profile.update', ['name' => $character->name]) }}" class="profile-preferences-form">
        @csrf
        @method('PUT')

        <fieldset class="field-stack">
            <legend>{{ __('character_profiles.visibility') }}</legend>
            <label class="checkbox-row">
                <input type="checkbox" name="show_guild" value="1" @checked(old('show_guild', $preference->show_guild))>
                <span>{{ __('character_profiles.show_guild') }}</span>
            </label>
            <p class="muted">{{ __('character_profiles.guild_help') }}</p>
        </fieldset>

        <fieldset class="field-stack">
            <legend>{{ __('character_profiles.main_character') }}</legend>
            <label class="checkbox-row">
                <input type="checkbox" name="is_main_character" value="1" @checked(old('is_main_character', $preference->is_main_character))>
                <span>{{ __('character_profiles.main_character') }}</span>
            </label>
            <p class="muted">{{ __('character_profiles.main_character_help') }}</p>
        </fieldset>

        <div class="action-row">
            <button type="submit">{{ __('character_profiles.save') }}</button>
            <a class="button button-secondary" href="{{ route('account.overview', ['locale' => app()->getLocale()]) }}">{{ __('character_profiles.back') }}</a>
        </div>
    </form>
@endsection
