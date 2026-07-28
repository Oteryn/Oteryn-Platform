@if ($context !== null)
    <aside class="catalog-context" aria-label="{{ __('game_catalog.context.label') }}">
        <span><strong>{{ __('game_catalog.context.profile') }}:</strong> {{ $context->profile_name }}</span>
        <span><strong>{{ __('game_catalog.context.target_release') }}:</strong> {{ $context->target_release_label }}</span>
        <span><strong>{{ __('game_catalog.context.snapshot') }}:</strong> #{{ $context->snapshot_id }}</span>
        <span><strong>{{ __('game_catalog.context.generated_at') }}:</strong> {{ $context->generated_at }}</span>
    </aside>
@else
    <div class="empty-state" role="status">
        <strong>{{ __('game_catalog.unavailable.title') }}</strong>
        <p>{{ __('game_catalog.unavailable.help') }}</p>
    </div>
@endif
