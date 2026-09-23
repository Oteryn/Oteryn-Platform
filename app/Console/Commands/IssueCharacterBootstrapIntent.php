<?php

namespace App\Console\Commands;

use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentIssuer;
use Illuminate\Console\Command;
use Throwable;

final class IssueCharacterBootstrapIntent extends Command
{
    protected $signature = 'game-auth:character-bootstrap-intent:issue
        {--identity-id= : Persisted Platform Identity primary key used for server-side AccountId resolution}
        {--operation-id= : Stable canonical UUID for this semantic operation}
        {--target-world-id= : Canonical target WorldId UUID}
        {--profile-revision= : Character interpretation profile revision}
        {--ruleset-revision= : Character ruleset revision}
        {--content-revision= : Character content revision}
        {--starter-template-revision= : Character starter-template revision}';

    protected $description = 'Issue or exactly reconcile one Platform-owned operator Character bootstrap intent.';

    public function handle(CharacterBootstrapIntentIssuer $issuer): int
    {
        $identity = $this->option('identity-id');
        if (! is_string($identity) || preg_match('/\A[1-9][0-9]*\z/D', $identity) !== 1) {
            $this->components->error('--identity-id must be a positive persisted Platform Identity id.');

            return self::FAILURE;
        }

        $operationId = $this->optionString('operation-id');
        $targetWorldId = $this->optionString('target-world-id');
        $profileRevision = $this->optionString('profile-revision');
        $rulesetRevision = $this->optionString('ruleset-revision');
        $contentRevision = $this->optionString('content-revision');
        $starterTemplateRevision = $this->optionString('starter-template-revision');
        if (in_array(null, [$operationId, $targetWorldId, $profileRevision, $rulesetRevision, $contentRevision, $starterTemplateRevision], true)) {
            return self::FAILURE;
        }

        try {
            $intent = $issuer->issue((int) $identity, $operationId, [
                'target_world_id' => $targetWorldId,
                'profile_revision' => $profileRevision,
                'ruleset_revision' => $rulesetRevision,
                'content_revision' => $contentRevision,
                'starter_template_revision' => $starterTemplateRevision,
            ]);
            $this->line(json_encode($intent, JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR));

            return self::SUCCESS;
        } catch (Throwable $exception) {
            $this->components->error($exception->getMessage());

            return self::FAILURE;
        }
    }

    private function optionString(string $name): ?string
    {
        $value = $this->option($name);
        if (! is_string($value) || $value === '') {
            $this->components->error("--{$name} must be a non-empty string.");

            return null;
        }

        return $value;
    }
}
