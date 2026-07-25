@extends('errors.layout')

@section('title', __('public.errors.503_title'))
@section('code', '503')
@section('heading', __('public.errors.503_heading'))
@section('message', __('public.errors.503_body'))

@section('actions')
    <a class="button" href="{{ route('home') }}">{{ __('public.errors.try_public_site') }}</a>
@endsection
