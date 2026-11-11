@extends('identity.layout')

@section('title', __('portal.identity.oauth_title'))

@section('portal-family', 'identity')

@section('content')
    <h1>{{ __('portal.identity.oauth_title') }}</h1>

    <p>
        <strong>{{ $client->name }}</strong> {{ __('portal.identity.oauth_request') }}
    </p>

    @if (count($scopes) > 0)
        <ul>
            @foreach ($scopes as $scope)
                <li>{{ $scope->description }}</li>
            @endforeach
        </ul>
    @endif

    <div class="action-row oauth-actions">
        <form method="POST" action="{{ route('passport.authorizations.approve') }}">
            @csrf
            <input type="hidden" name="state" value="{{ $request->state }}">
            <input type="hidden" name="client_id" value="{{ $client->getKey() }}">
            <input type="hidden" name="auth_token" value="{{ $authToken }}">
            <button type="submit">{{ __('portal.identity.oauth_continue') }}</button>
        </form>

        <form method="POST" action="{{ route('passport.authorizations.deny') }}">
            @csrf
            @method('DELETE')
            <input type="hidden" name="state" value="{{ $request->state }}">
            <input type="hidden" name="client_id" value="{{ $client->getKey() }}">
            <input type="hidden" name="auth_token" value="{{ $authToken }}">
            <button type="submit">{{ __('portal.identity.cancel') }}</button>
        </form>
    </div>
@endsection
