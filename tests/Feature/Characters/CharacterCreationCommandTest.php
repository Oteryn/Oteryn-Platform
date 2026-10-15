<?php

namespace Tests\Feature\Characters;

use App\CharacterAuthority\CharacterCommandWire;
use App\Characters\Actions\CreateCharacter;
use App\Characters\Exceptions\CharacterBindingNotReady;
use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

/** Web character creation through Game Character Authority commands (Game contract §14 step 3). */
final class CharacterCreationCommandTest extends TestCase
{
    use RefreshDatabase;

    private const PEER = 'CN=oteryn-game-character-commands';

    private const SOURCE = 'oteryn:character-authority:primary';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const OTHER_WORLD = '01934f10-7c02-7001-805b-3b1122334402';

    private const CHARACTER = '01934f10-7c04-7001-805b-3b1122334401';

    private const SUBMISSION = '5b0d6f8e-2a1c-4d3b-9e8f-7a6b5c4d3e2f';

    public function test_web_creation_is_disabled_by_default_even_when_commands_are_enabled(): void
    {
        self::assertFalse(config('character-authority.create.enabled'));
        $this->enableCommands();
        $this->world();
        $this->login($this->identity());

        $form = $this->get('/account/characters/create');
        $form->assertOk();
        $form->assertDontSee('name="world"', false);
        $form->assertDontSee('name="submission"', false);

        $this->post('/account/characters', $this->input())
            ->assertSessionHasErrors(['character' => 'Character creation is temporarily unavailable.']);
        self::assertSame(0, DB::table('character_command_intents')->count());
    }

    public function test_web_creation_stays_unavailable_while_the_command_surface_is_off(): void
    {
        config(['character-authority.create.enabled' => true]);
        $this->world();
        $this->login($this->identity());

        $this->post('/account/characters', $this->input())
            ->assertSessionHasErrors(['character' => 'Character creation is temporarily unavailable.']);
        self::assertSame(0, DB::table('character_command_intents')->count());
    }

    public function test_submission_records_one_create_intent_for_the_signed_in_account(): void
    {
        $this->enable();
        $this->world();
        $identity = $this->identity();
        $this->login($identity);

        $form = $this->get('/account/characters/create');
        $form->assertOk();
        $form->assertSee('name="world"', false);
        $form->assertSee('name="submission"', false);
        $form->assertSee('value="'.self::WORLD.'"', false);
        $form->assertDontSee('name="account_id"', false);

        $first = $this->post('/account/characters', [...$this->input(), 'account_id' => '0190f2a1-3b4c-7d5e-8f60-718293a4b5c7']);
        $second = $this->post('/account/characters', $this->input());

        self::assertSame(1, DB::table('character_command_intents')->count());
        $operationId = $this->latestIntent('operation_id');
        $first->assertRedirect(route('account.characters.requests.show', ['operation' => $operationId, 'locale' => 'en']));
        $second->assertRedirect(route('account.characters.requests.show', ['operation' => $operationId, 'locale' => 'en']));

        $intent = json_decode($this->latestIntent('intent_json'), true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($intent);
        self::assertSame(CharacterCommandWire::CREATE, $intent['command']);
        self::assertSame($identity->account_id, $intent['account_id']);
        self::assertSame(self::WORLD, $intent['target_world_id']);
        self::assertSame('Alice Moon', $intent['requested_name']);

        $pending = $this->get(route('account.characters.requests.show', ['operation' => $operationId]));
        $pending->assertOk();
        $pending->assertSee('data-outcome="pending"', false);
        $pending->assertSee('Your character request is being processed.');
    }

    public function test_the_selected_locale_carries_to_the_translated_result_page(): void
    {
        $this->enable();
        $this->world();
        $this->login($this->identity());

        $response = $this->post('/account/characters?locale=pl', $this->input());
        $operationId = $this->latestIntent('operation_id');
        $response->assertRedirect(route('account.characters.requests.show', ['operation' => $operationId, 'locale' => 'pl']));

        $page = $this->get(route('account.characters.requests.show', ['operation' => $operationId, 'locale' => 'pl']));
        $page->assertOk();
        $page->assertSee('Zgłoszenie postaci');
        $page->assertSee('Twoje zgłoszenie postaci jest przetwarzane.');
        $page->assertDontSee('Your character request is being processed.');
    }

    public function test_the_same_submission_with_other_details_is_refused(): void
    {
        $this->enable();
        $this->world();
        $this->login($this->identity());

        $this->post('/account/characters', $this->input())->assertRedirect();
        $this->post('/account/characters', [...$this->input(), 'name' => 'Bob Stone'])
            ->assertSessionHasErrors(['character' => 'The character creation input is invalid.']);
        self::assertSame(1, DB::table('character_command_intents')->count());
    }

    public function test_unknown_world_or_missing_submission_is_refused_without_an_intent(): void
    {
        $this->enable();
        $this->world();
        $this->login($this->identity());

        $this->post('/account/characters', [...$this->input(), 'world' => self::OTHER_WORLD])
            ->assertSessionHasErrors(['character' => 'The character creation input is invalid.']);
        $this->post('/account/characters', [...$this->input(), 'submission' => null])
            ->assertSessionHasErrors(['character' => 'The character creation input is invalid.']);
        $this->post('/account/characters', [...$this->input(), 'name' => 'OterynSupport'])
            ->assertSessionHasErrors('name');
        self::assertSame(0, DB::table('character_command_intents')->count());
    }

    public function test_termination_committed_after_the_identity_was_loaded_blocks_the_intent(): void
    {
        $this->enable();
        $this->world();
        $identity = $this->identity();
        DB::table('identities')->where('id', $identity->getKey())->update(['terminated_at' => now()]);

        $refused = false;
        try {
            app(CreateCharacter::class)->execute($identity, 'Alice Moon', 9, 1, self::WORLD, self::SUBMISSION);
        } catch (CharacterBindingNotReady) {
            $refused = true;
        }
        self::assertTrue($refused);
        self::assertNull($identity->terminated_at);
        self::assertSame(0, DB::table('character_command_intents')->count());
    }

    public function test_committed_receipt_shows_the_created_character(): void
    {
        $this->enable();
        $this->world();
        $identity = $this->identity();
        $this->login($identity);
        // A native account has no legacy binding. Enter through the real Account Center action.
        $this->get('/account?locale=pl')->assertOk()
            ->assertSee(route('account.characters.create', ['locale' => 'pl']), false);
        $this->get('/account/characters/create?locale=pl')->assertOk()
            ->assertSee('name="submission"', false);
        $operationId = $this->submit();

        $this->publish($this->membersJson([
            ...$this->receiptHead($operationId, 'COMMITTED', 'CHAR_CMD_OK'),
            'character_id' => self::CHARACTER,
            'account_id' => $identity->account_id,
            'world_id' => self::WORLD,
            'name' => 'Alice Moon',
            'character_revision' => '1',
            'projection_epoch' => '1',
            'projection_revision' => '7',
            'recovery_generation' => '1',
            'decided_at' => '1790000002',
        ]))->assertOk();

        $page = $this->get(route('account.characters.requests.show', ['operation' => $operationId]));
        $page->assertOk();
        $page->assertSee('data-outcome="created"', false);
        $page->assertSee('Character Alice Moon created.');
    }

    public function test_rejected_receipts_resolve_to_user_visible_outcomes(): void
    {
        $this->enable();
        $this->world();
        $this->login($this->identity());

        foreach ([
            'CHAR_CMD_NAME_UNAVAILABLE' => 'That character name is not available.',
            'CHAR_CMD_ACCOUNT_CHARACTER_LIMIT' => 'This account already has the maximum number of active characters.',
            'CHAR_CMD_WORLD_NOT_ACCEPTED' => 'That world is not accepting new characters.',
            'CHAR_CMD_INTENT_EXPIRED' => 'The request expired before it was processed. Please submit it again.',
            'CHAR_CMD_OPERATION_CONFLICT' => 'Character creation could not be completed. Please try again later.',
        ] as $resultCode => $message) {
            $operationId = $this->submit((string) Str::uuid());
            $this->publish($this->membersJson([
                ...$this->receiptHead($operationId, 'REJECTED', $resultCode),
                'recovery_generation' => '1',
                'decided_at' => '1790000002',
            ]))->assertOk();

            $page = $this->get(route('account.characters.requests.show', ['operation' => $operationId]));
            $page->assertOk();
            $page->assertSee('data-outcome="rejected"', false);
            $page->assertSee($message);
        }
    }

    public function test_another_identity_cannot_read_the_request(): void
    {
        $this->enable();
        $this->world();
        $this->login($this->identity());
        $operationId = $this->submit();
        $this->post('/logout');

        $this->login($this->identity());
        $this->get(route('account.characters.requests.show', ['operation' => $operationId]))->assertNotFound();
        $this->get(route('account.characters.requests.show', ['operation' => 'not-an-operation']))->assertNotFound();
    }

    private function submit(string $submission = self::SUBMISSION): string
    {
        $this->post('/account/characters', [...$this->input(), 'submission' => $submission])->assertRedirect();

        return $this->latestIntent('operation_id');
    }

    private function latestIntent(string $column): string
    {
        $value = DB::table('character_command_intents')->orderByDesc('id')->value($column);
        self::assertIsString($value);

        return $value;
    }

    private function enable(): void
    {
        $this->enableCommands();
        config(['character-authority.create.enabled' => true]);
    }

    private function enableCommands(): void
    {
        config([
            'character-authority.commands.enabled' => true,
            'character-authority.commands.identities' => json_encode([self::PEER]),
            'character-authority.commands.source_authority' => self::SOURCE,
        ]);
    }

    private function world(): void
    {
        $world = GameWorld::query()->create([
            'slug' => 'acceptance',
            'name' => 'Acceptance',
            'region' => 'TEST',
            'status' => GameWorldStatus::Online,
            'login_enabled' => true,
            'game_host' => '127.0.0.1',
            'game_port' => 7172,
        ]);
        DB::table('game_worlds')->where('id', $world->id)->update(['world_id' => self::WORLD]);
    }

    /** @return array<string, int|string|null> */
    private function input(): array
    {
        return [
            'name' => '  aLiCe   moon  ',
            'vocation' => 9,
            'sex' => 1,
            'world' => self::WORLD,
            'submission' => self::SUBMISSION,
        ];
    }

    /** @return TestResponse<Response> */
    private function publish(string $receipt): TestResponse
    {
        return $this->call('POST', CharacterCommandWire::RECEIPT_PATH, [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER,
        ], $receipt);
    }

    /** @return array<string, int|string> */
    private function receiptHead(string $operationId, string $outcome, string $resultCode): array
    {
        return [
            'contract_version' => 1,
            'operation' => 'PublishCharacterCommandReceiptV1',
            'source_authority' => self::SOURCE,
            'command' => CharacterCommandWire::CREATE,
            'operation_id' => $operationId,
            'outcome' => $outcome,
            'result_code' => $resultCode,
        ];
    }

    /** @param array<string, int|string> $members */
    private function membersJson(array $members): string
    {
        return json_encode($members, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }

    private function identity(): Identity
    {
        return Identity::query()->create([
            'email' => uniqid('character-', true).'@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function login(Identity $identity): void
    {
        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'Correct-Horse-9!Battery',
        ])->assertRedirect(route('home'));

        $this->assertAuthenticatedAs($identity, 'web');
    }
}
