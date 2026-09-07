{{-- Decorative UI icon: names are chosen by templates, not untrusted markup. --}}
<svg class="realm-icon" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
    @switch($icon ?? 'compass')
        @case('book')
        @case('chronicles')
            <path d="M16 8c-4-3-8-3-12-2v20c4-1 8-1 12 2 4-3 8-3 12-2V6c-4-1-8-1-12 2Zm0 0v20M8 11h4m-4 5h4m8-5h4m-4 5h4"/>
            @break
        @case('community')
            <circle cx="16" cy="9" r="4"/><path d="M8 27v-4a8 8 0 0 1 16 0v4H8ZM7 10a3 3 0 0 0 0 6m18-6a3 3 0 0 1 0 6M3 26v-4a6 6 0 0 1 3-5m23 9v-4a6 6 0 0 0-3-5"/>
            @break
        @case('download')
            <path d="M16 3v18m-6-6 6 6 6-6M5 22v6h22v-6"/>
            @break
        @case('shield')
            <path d="m16 3 11 4v9c0 6-7 11-11 14C12 27 5 22 5 16V7l11-4Zm0 5v16m-5-11h10"/>
            @break
        @default
            <circle cx="16" cy="16" r="11"/><path d="M16 1v6m0 18v6M1 16h6m18 0h6M23 9l-9 5-5 9 9-5 5-9Z"/>
    @endswitch
</svg>
