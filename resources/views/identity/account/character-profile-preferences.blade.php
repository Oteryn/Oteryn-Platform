@extends('identity.layout')

@section('title', __('character_profiles.title'))

@section('content')
    <header class="page-header">
        <p class="eyebrow">{{ __('character_profiles.eyebrow') }}</p>
        <h1>{{ $character->name }}</h1>
        <p class="muted">{{ __('character_profiles.description') }}</p>
    </header>

    @if ($errors->any())
        <div class="notice alert-danger" role="alert">
            <ul>
                @foreach ($errors->all() as $error)
                    <li>{{ $error }}</li>
                @endforeach
            </ul>
        </div>
    @endif

    <form method="POST" action="{{ route('account.characters.profile.update', ['name' => $character->name]) }}" class="panel">
        @csrf
        @method('PUT')

        <div class="field-stack">
            <label for="public_comment">{{ __('character_profiles.comment') }}</label>
            <textarea id="public_comment" name="public_comment" rows="6" maxlength="500" aria-describedby="public-comment-help">{{ old('public_comment', $preference->public_comment) }}</textarea>
            <p id="public-comment-help" class="muted">{{ __('character_profiles.comment_help') }}</p>
        </div>

        <fieldset class="field-stack">
            <legend>{{ __('character_profiles.visibility') }}</legend>
            @foreach ([
                'show_account_association',
                'show_status',
                'show_guild',
                'show_house',
                'show_skills',
                'show_deaths',
                'show_kills',
            ] as $field)
                <label class="checkbox-row">
                    <input type="checkbox" name="{{ $field }}" value="1" @checked(old($field, $preference->{$field}))>
                    <span>{{ __('character_profiles.'.$field) }}</span>
                </label>
            @endforeach
        </fieldset>

        <div class="notice" role="note">
            {{ __('character_profiles.account_privacy_notice') }}
        </div>

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
            <a class="button button-secondary" href="{{ route('account.overview') }}">{{ __('character_profiles.back') }}</a>
        </div>
    </form>
@endsection
