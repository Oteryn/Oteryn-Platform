@extends('errors.layout')

@section('title', __('public.errors.404_title'))
@section('code', '404')
@section('heading', __('public.errors.404_heading'))
@section('message', __('public.errors.404_body'))

@section('actions')
    <a class="button" href="{{ route('home') }}">{{ __('public.errors.back_home') }}</a>
    <a class="button button-secondary" href="{{ route('news.index') }}">{{ __('public.errors.browse_news') }}</a>
@endsection
