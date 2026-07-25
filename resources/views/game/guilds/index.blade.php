@extends('game.layout')

@section('title', 'Guilds')
@section('page-class', 'page-shell-wide')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Community</p>
        <h1>Guild directory</h1>
        <p class="muted">Guilds are listed alphabetically. Active member totals include only currently listable characters.</p>
    </div>

    <div class="card">
        <div class="table-region" tabindex="0" aria-label="Guild directory table, horizontally scrollable on small screens">
            <table class="table-compact">
                <thead>
                <tr>
                    <th scope="col">Guild</th>
                    <th scope="col">Active members</th>
                </tr>
                </thead>
                <tbody>
                @forelse ($guilds as $guild)
                    <tr>
                        <td><a href="{{ route('game.guilds.show', ['name' => $guild->name]) }}">{{ $guild->name }}</a></td>
                        <td>{{ $guild->active_member_count }}</td>
                    </tr>
                @empty
                    <tr><td colspan="2">No guilds found.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>

        @if ($guilds->hasPages())
            <nav class="pagination" aria-label="Guild directory pages">
                @if ($guilds->onFirstPage())
                    <span class="muted">Previous</span>
                @else
                    <a href="{{ $guilds->previousPageUrl() }}">Previous</a>
                @endif
                <span>Page {{ $guilds->currentPage() }} of {{ $guilds->lastPage() }}</span>
                @if ($guilds->hasMorePages())
                    <a href="{{ $guilds->nextPageUrl() }}">Next</a>
                @else
                    <span class="muted">Next</span>
                @endif
            </nav>
        @endif
    </div>
@endsection
