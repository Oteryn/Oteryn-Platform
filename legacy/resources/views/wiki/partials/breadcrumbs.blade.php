<nav class="wiki-breadcrumbs" aria-label="{{ __('public.wiki.breadcrumbs') }}">
    <ol>
        @foreach ($breadcrumbs as $breadcrumb)
            <li>
                @if ($breadcrumb->url !== null)
                    <a href="{{ $breadcrumb->url }}">{{ $breadcrumb->label }}</a>
                @else
                    <span aria-current="page">{{ $breadcrumb->label }}</span>
                @endif
            </li>
        @endforeach
    </ol>
</nav>
