@extends('errors.layout')

@section('title', __('errors.429_title'))
@section('code', '429')
@section('heading', __('errors.429_heading'))
@section('message', __('errors.429_body'))

@section('actions')
    <a class="button" href="{{ route('identity.login.create', ['locale' => app()->getLocale()]) }}">{{ __('errors.return_to_sign_in') }}</a>
    <a class="button button-secondary" href="{{ route('home') }}">{{ __('errors.back_home') }}</a>
@endsection
