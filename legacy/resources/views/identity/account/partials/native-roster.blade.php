    <section class="account-roster" aria-labelledby="account-characters-heading" data-characters-state="{{ $overview['characters_state'] }}">
        <div class="section-heading">
            <div><p class="eyebrow">{{ __('portal.account.game_account') }}</p><h2 id="account-characters-heading">{{ __('portal.account.characters') }}</h2></div>
        </div>
        @if ($overview['characters_state'] === 'ready')
            @if (count($overview['characters']) > 0)
                <div class="character-roster">
                    @foreach ($overview['characters'] as $character)
                        @php($profilePreference = ($profilePreferences ?? collect())->get($character->characterId))
                        <article class="card character-roster-row">
                            <div class="character-monogram" aria-hidden="true">{{ mb_substr($character->name, 0, 1) }}</div>
                            <div class="character-roster-identity">
                                <h3>{{ $character->name }}</h3>
                                @if ($profilePreference?->is_main_character)<span class="badge badge-success" data-main-character>{{ __('character_profiles.main_badge') }}</span>@endif
                            </div>
                            <div class="action-row">
                                <a class="button button-ghost" href="{{ route('account.characters.profile.edit', ['name' => $character->name, 'locale' => app()->getLocale()]) }}">{{ __('character_profiles.manage') }}</a>
                            </div>
                        </article>
                    @endforeach
                </div>
            @else
                <div class="empty-state"><p>{{ __('portal.account.empty') }}</p>
                    @if ($overview['character_creation_allowed'])<a class="button" href="{{ route('account.characters.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.account.first') }}</a>@endif
                </div>
            @endif
        @else
            <p class="muted">{{ __('portal.account.native_'.$overview['characters_state']) }}</p>
        @endif
        @unless ($overview['character_creation_allowed'])
            <p class="muted">{{ __('portal.account.creation_unavailable') }}</p>
        @endunless
    </section>
