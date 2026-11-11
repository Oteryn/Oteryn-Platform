@extends('errors.layout')

@section('title', __('errors.409_title'))
@section('code', '409')
@section('heading', __('errors.409_heading'))
@section('message', __('errors.409_body'))

@section('actions')
    <a class="button" href="{{ route('home') }}">{{ __('errors.back_home') }}</a>
@endsection
