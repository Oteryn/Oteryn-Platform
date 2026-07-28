@extends('identity.layout')

@section('title', 'Confirm email change')

@section('content')
    <div class="identity-heading">
        <p class="eyebrow">Primary email</p>
        <h1>Confirm the new email address</h1>
        <p>This single-use action changes the sign-in email and revokes every existing web session and game authorization.</p>
    </div>

    <form method="POST" action="{{ route('identity.email-change.confirm', ['token' => $token]) }}" class="stacked-form">
        @csrf
        <button type="submit">Confirm email change</button>
    </form>
@endsection
