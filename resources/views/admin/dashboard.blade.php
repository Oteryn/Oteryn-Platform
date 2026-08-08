@extends('admin.layout')

@inject('adminAuthorization', 'App\Admin\AdminAuthorization')

@section('title', 'Oteryn Admin')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Administrator console</p>
        <h1>Administration</h1>
        <p class="muted">Privileged administration surface. Every linked capability remains independently permission-protected.</p>
    </div>

    <div class="card-grid">
        <article class="card">
            <h2>Content</h2>
            <p class="muted">Manage published news and public managed pages.</p>
            <div class="action-row">
                <a class="button button-secondary" href="{{ route('admin.news.index') }}">Manage news</a>
                <a class="button button-secondary" href="{{ route('admin.pages.index') }}">Manage pages</a>
            </div>
        </article>
        <article class="card">
            <h2>Access</h2>
            <p class="muted">Review and manage administrator role assignments.</p>
            <a class="button button-secondary" href="{{ route('admin.roles.index') }}">Manage roles</a>
        </article>
        <article class="card">
            <h2>Operations</h2>
            <p class="muted">Inspect bounded privileged administration audit events.</p>
            <a class="button button-secondary" href="{{ route('admin.audit.index') }}">View audit</a>
        </article>
        @if (auth()->user() instanceof \App\Identity\Models\Identity && $adminAuthorization->allows(auth()->user(), \App\Admin\AdminPermission::MANAGE_PORTAL_SETTINGS))
            <article class="card">
                <h2>{{ __('homepage_templates.admin.dashboard_title') }}</h2>
                <p class="muted">{{ __('homepage_templates.admin.dashboard_help') }}</p>
                <a class="button button-secondary" href="{{ route('admin.homepage-templates.index') }}">{{ __('homepage_templates.admin.dashboard_title') }}</a>
            </article>
        @endif
    </div>
@endsection
