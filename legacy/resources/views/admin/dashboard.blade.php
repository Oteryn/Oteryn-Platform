@extends('admin.layout')

@section('title', 'Oteryn Admin')

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal_art.admin.workspace') }}</p>
        <h1>{{ __('portal_art.admin.console') }}</h1>
        <p class="muted">{{ __('portal_art.admin.workspace_help') }}</p>
    </div>
    <div class="admin-task-grid">
        @include('admin.partials.navigation', ['asDashboard' => true])
    </div>
@endsection
