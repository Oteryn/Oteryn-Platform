<?php

namespace App\Console\Commands;

use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentService;
use Illuminate\Console\Command;
use Throwable;

final class IssueCharacterBootstrapIntent extends Command
{
    protected $signature = 'game-auth:character-bootstrap-intent:issue
        {identity-id : Platform Identity database identifier used only for canonical lookup}
        {--operation-id= : Stable canonical UUID correlation identity}
        {--target-world-id= : Bounded Game world authority identifier}
        {--profile-revision= : Positive interpretation profile revision}
        {--ruleset-revision= : Positive interpretation ruleset revision}
        {--content-revision= : Positive interpretation content revision}
        {--starter-template-revision= : Positive starter template revision}';

    protected $description = 'Issue or reconcile one immutable operator Character bootstrap intent.';

    public function handle(CharacterBootstrapIntentService $service): int
    {
        $identity = $this->argument('identity-id');
        if (preg_match('/^[1-9][0-9]*$/', $identity) !== 1) {
            $this->components->error('Identity id must be a positive Platform identifier.');

            return self::FAILURE;
        }
        $values = [];
        foreach (['operation-id', 'target-world-id', 'profile-revision', 'ruleset-revision', 'content-revision', 'starter-template-revision'] as $name) {
            $value = $this->option($name);
            if (! is_string($value) || trim($value) === '') {
                $this->components->error("--{$name} is required.");

                return self::FAILURE;
            }
            $values[$name] = trim($value);
        }
        try {
            $payload = $service->issue((int) $identity, $values['operation-id'], $values['target-world-id'], [
                'profile_revision' => $values['profile-revision'],
                'ruleset_revision' => $values['ruleset-revision'],
                'content_revision' => $values['content-revision'],
                'starter_template_revision' => $values['starter-template-revision'],
            ]);
            $this->line(json_encode($payload, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));

            return self::SUCCESS;
        } catch (Throwable $exception) {
            $this->components->error($exception->getMessage());

            return self::FAILURE;
        }
    }
}
