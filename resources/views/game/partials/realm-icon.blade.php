<svg class="realm-icon" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
    @switch($icon)
        @case('book')
            <path d="M4 6h8l4 3 4-3h8v20h-8l-4 2-4-2H4zM16 9v19M8 11h4m-4 5h4m8-5h4m-4 5h4"/>
            @break
        @case('community')
            <circle cx="16" cy="9" r="4"/><path d="M9 27v-5a7 7 0 0 1 14 0v5zM7 9a3 3 0 0 0 0 6m18-6a3 3 0 0 1 0 6M6 19a5 5 0 0 0-3 5v3h3m20-8a5 5 0 0 1 3 5v3h-3"/>
            @break
        @case('download')
            <path d="M16 3v17m-6-6 6 6 6-6M5 22v6h22v-6"/>
            @break
        @case('shield')
            <path d="m16 3 11 5v8c0 6-11 13-11 13S5 22 5 16V8zM16 9v11m-4-7h8"/>
            @break
        @case('chronicles')
            <path d="M8 4h16v24H8zM12 9h8m-8 5h8m-8 5h5M5 8H3v21h17"/>
            @break
        @default
            <circle cx="16" cy="16" r="11"/><path d="M16 1v7m0 16v7M1 16h7m16 0h7m-11-19-7 3-3 7 7-3z"/>
    @endswitch
</svg>
