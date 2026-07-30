@extends('errors.layout')

@section('title', __('errors.500_title'))
@section('code', '500')
@section('heading', __('errors.500_heading'))
@section('message', __('errors.500_body'))

@section('actions')
    <a class="button" href="{{ route('home') }}">{{ __('errors.back_home') }}</a>
    <a class="button button-secondary" href="{{ route('news.index') }}">{{ __('errors.browse_news') }}</a>
@endsection
