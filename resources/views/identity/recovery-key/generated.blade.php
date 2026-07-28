@extends('identity.layout')

@section('title', 'Recovery key generated')

@section('content')
    <div class="identity-heading">
        <p class="eyebrow">High-assurance recovery</p>
        <h1>Store this recovery key now</h1>
        <p>The key is displayed once. Store it offline. Generating another key or revoking this key makes it unusable.</p>
    </div>

    <div class="recovery-codes" role="status" aria-label="New recovery key">
        <code>{{ $recoveryKey }}</code>
    </div>

    <p class="muted">Oteryn stores only a keyed verifier. Support and administrators cannot display the key again.</p>
    <div class="action-row">
        <a class="button" href="{{ route('identity.account-security.show') }}">Return to account security</a>
    </div>
@endsection
