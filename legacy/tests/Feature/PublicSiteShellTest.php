<?php

namespace Tests\Feature;

use Tests\TestCase;

class PublicSiteShellTest extends TestCase
{
    public function test_home_uses_shared_shell_with_public_navigation_and_character_search(): void
    {
        $this->get(route('home'))
            ->assertOk()
            ->assertSee('Oteryn Platform')
            ->assertSee('Find a character')
            ->assertSee('Search by exact character name.')
            ->assertSee(route('home'), false)
            ->assertSee(route('game.online.index'), false)
            ->assertSee(route('game.highscores.index'), false)
            ->assertSee(route('game.servers.index'), false)
            ->assertSee(route('game.characters.search'), false);
    }

    public function test_character_search_redirects_exact_name_to_existing_profile_route(): void
    {
        $this->get(route('game.characters.search', ['name' => 'Active Knight']))
            ->assertRedirect(route('game.characters.show', ['name' => 'Active Knight']));
    }

    public function test_character_search_requires_a_name(): void
    {
        $this->get(route('game.characters.search'))
            ->assertSessionHasErrors('name');
    }
}
