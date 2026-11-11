@extends('errors.layout')

@section('title', 'Access denied')
@section('code', '403')
@section('heading', 'You do not have access to this page')
@section('message', 'Your current account or session is not authorized for this action. No changes were made.')

@section('actions')
    <a class="button" href="{{ route('home') }}">Go to public site</a>
    @auth
        <a class="button button-secondary" href="{{ route('identity.mfa.settings') }}">Account security</a>
    @else
        <a class="button button-secondary" href="{{ route('identity.login.create') }}">Sign in</a>
    @endauth
@endsection
