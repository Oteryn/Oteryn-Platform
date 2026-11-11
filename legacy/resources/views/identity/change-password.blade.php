@extends('identity.layout')

@section('title', __('portal.identity.change_title'))
@section('error-title', __('portal.identity.change_error'))

@section('portal-family', 'security')

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal.identity.security') }}</p>
        <h1>{{ __('portal.identity.change_heading') }}</h1>
        <p class="muted">{{ __('portal.identity.change_intro') }}</p>
    </div>

    <form class="form-stack" method="POST" action="{{ route('identity.password.change.update', ['locale' => app()->getLocale()]) }}">
        @csrf
        @method('PUT')
        <div class="form-field">
            <label for="current_password">{{ __('portal.identity.current_password') }}</label>
            <input id="current_password" name="current_password" type="password" autocomplete="current-password" maxlength="1024" required autofocus @error('current_password') aria-invalid="true" aria-describedby="error-current-password" @enderror>
        </div>
        <div class="form-field">
            <label for="password">{{ __('portal.identity.new_password') }}</label>
            <input id="password" name="password" type="password" autocomplete="new-password" maxlength="1024" required @error('password') aria-invalid="true" aria-describedby="error-password" @enderror>
        </div>
        <div class="form-field">
            <label for="password_confirmation">{{ __('portal.identity.confirm_new') }}</label>
            <input id="password_confirmation" name="password_confirmation" type="password" autocomplete="new-password" maxlength="1024" required @error('password_confirmation') aria-invalid="true" aria-describedby="error-password-confirmation" @enderror>
        </div>
        <button type="submit">{{ __('portal.identity.change') }}</button>
    </form>
@endsection
