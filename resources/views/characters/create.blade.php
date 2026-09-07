@extends('identity.layout')

@section('title', __('portal.create.title'))
@section('error-title', __('portal.create.error'))

@section('portal-family', 'character-create')

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal.create.eyebrow') }}</p>
        <h1>{{ __('portal.create.heading') }}</h1>
        <p class="muted">{{ __('portal.create.intro') }}</p>
    </div>

    <div class="character-creation-layout">
    <form class="form-stack" method="POST" action="{{ route('account.characters.store', ['locale' => app()->getLocale()]) }}">
        @csrf
        <div class="form-field">
            <label for="name">{{ __('portal.create.name') }}</label>
            <input id="name" name="name" type="text" value="{{ old('name') }}" maxlength="255" aria-describedby="character-name-help @error('name') error-name @enderror" @error('name') aria-invalid="true" @enderror autocomplete="off" required autofocus>
            <p id="character-name-help" class="form-help">{{ __('portal.create.name_help') }}</p>
        </div>
        <div class="form-field">
            <label for="vocation">{{ __('portal.create.vocation') }}</label>
            <select id="vocation" name="vocation" required @error('vocation') aria-invalid="true" aria-describedby="error-vocation" @enderror>
                <option value="1" @selected((string) old('vocation') === '1')>Sorcerer</option>
                <option value="2" @selected((string) old('vocation') === '2')>Druid</option>
                <option value="3" @selected((string) old('vocation') === '3')>Paladin</option>
                <option value="4" @selected((string) old('vocation') === '4')>Knight</option>
                <option value="9" @selected((string) old('vocation') === '9')>Monk</option>
            </select>
        </div>
        <div class="form-field">
            <label for="sex">{{ __('portal.create.sex') }}</label>
            <select id="sex" name="sex" required @error('sex') aria-invalid="true" aria-describedby="error-sex" @enderror>
                <option value="0" @selected((string) old('sex') === '0')>{{ __('portal.create.female') }}</option>
                <option value="1" @selected((string) old('sex', '1') === '1')>{{ __('portal.create.male') }}</option>
            </select>
        </div>
        <button type="submit">{{ __('portal.create.submit') }}</button>
    </form>

    <aside class="creation-guide" aria-label="{{ __('portal.account.workspace') }}"><img src="{{ asset('images/oteryn-sigil.svg') }}" width="160" height="160" alt=""><h2>{{ __('portal.home.journey') }}</h2><p>{{ __('portal.create.name_help') }}</p><a href="{{ route('account.overview', ['locale' => app()->getLocale()]) }}">{{ __('portal.account.characters') }} <span aria-hidden="true">→</span></a></aside>
    </div>
    <div class="identity-links">
        <a href="{{ route('identity.mfa.settings', ['locale' => app()->getLocale()]) }}">{{ __('portal.create.security') }}</a>
        <a href="{{ route('home') }}">{{ __('portal.create.back') }}</a>
    </div>
@endsection
