@extends('identity.layout')

@section('title', 'Recover email change')

@section('content')
    <div class="identity-heading">
        <p class="eyebrow">Primary email recovery</p>
        <h1>Cancel or recover the email change</h1>
        <p>This single-use action cancels a pending change or restores the previous address during the recovery window. A completed recovery revokes every existing session.</p>
    </div>

    <form method="POST" action="{{ route('identity.email-change.recover', ['token' => $token]) }}" class="stacked-form">
        @csrf
        <button class="button button-danger" type="submit">Cancel or recover email change</button>
    </form>
@endsection
