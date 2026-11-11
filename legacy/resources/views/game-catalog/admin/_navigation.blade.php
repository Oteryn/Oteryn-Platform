<nav class="admin-subnav" aria-label="Game Catalog administration">
    <a href="{{ route('admin.game-catalog.index') }}" @if(request()->routeIs('admin.game-catalog.index')) aria-current="page" @endif>Overview</a>
    <a href="{{ route('admin.game-catalog.snapshots.index') }}" @if(request()->routeIs('admin.game-catalog.snapshots.*')) aria-current="page" @endif>Snapshots</a>
    <a href="{{ route('admin.game-catalog.profiles.index') }}" @if(request()->routeIs('admin.game-catalog.profiles.*')) aria-current="page" @endif>Profiles</a>
    <a href="{{ route('admin.game-catalog.findings.index') }}" @if(request()->routeIs('admin.game-catalog.findings.*')) aria-current="page" @endif>Findings</a>
    <a href="{{ route('admin.game-catalog.diff.index') }}" @if(request()->routeIs('admin.game-catalog.diff.*')) aria-current="page" @endif>Diff</a>
</nav>
