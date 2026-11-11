<?php

namespace App\Console\Commands;

use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use Illuminate\Console\Command;
use Illuminate\Support\Facades\DB;
use LogicException;
use Throwable;

final class EnsureGameWorld extends Command
{
    protected $signature = 'game-auth:world:ensure
        {--id= : Positive Platform world identifier}
        {--slug= : Stable lower-case world slug}
        {--name= : Player-facing world name}
        {--region= : Bounded region label}
        {--host= : Authoritative game host or IP}
        {--port= : Authoritative game TCP port}
        {--status=online : World status}
        {--login-enabled=1 : Whether new native logins are enabled}';

    protected $description = 'Create or update one explicitly configured Oteryn World Registry route.';

    public function handle(): int
    {
        $id = $this->positiveIntegerOption('id');
        $port = $this->positiveIntegerOption('port');
        $slug = $this->stringOption('slug');
        $name = $this->stringOption('name');
        $region = $this->stringOption('region');
        $host = $this->stringOption('host');
        $status = $this->stringOption('status');
        $loginEnabled = $this->booleanOption('login-enabled');

        if ($id === null || $port === null || $port > 65535
            || $slug === null || $name === null || $region === null || $host === null || $status === null
            || $loginEnabled === null) {
            return self::FAILURE;
        }
        if (preg_match('/^[a-z0-9][a-z0-9-]{0,63}$/', $slug) !== 1) {
            $this->components->error('World slug must contain only lower-case letters, digits and hyphens, and be at most 64 characters.');

            return self::FAILURE;
        }
        if ($name === '' || mb_strlen($name) > 100) {
            $this->components->error('World name must be between 1 and 100 characters.');

            return self::FAILURE;
        }
        if ($region === '' || mb_strlen($region) > 32 || preg_match('/^[A-Za-z0-9_-]+$/', $region) !== 1) {
            $this->components->error('World region must be a simple 1 to 32 character label.');

            return self::FAILURE;
        }
        if (! $this->validHost($host)) {
            $this->components->error('World host must be a valid IPv4/IPv6 address or DNS hostname without a scheme or path.');

            return self::FAILURE;
        }
        $worldStatus = GameWorldStatus::tryFrom($status);
        if ($worldStatus === null) {
            $this->components->error('World status is not supported.');

            return self::FAILURE;
        }

        try {
            DB::transaction(function () use ($id, $slug, $name, $region, $host, $port, $worldStatus, $loginEnabled): void {
                $slugOwner = GameWorld::query()->where('slug', $slug)->lockForUpdate()->first();
                if ($slugOwner !== null && $slugOwner->id !== $id) {
                    throw new LogicException('World slug is already assigned to another world identifier.');
                }

                $world = GameWorld::query()->lockForUpdate()->find($id);
                if ($world === null) {
                    $world = new GameWorld;
                    $world->setAttribute('id', $id);
                }

                $world->fill([
                    'slug' => $slug,
                    'name' => $name,
                    'region' => $region,
                    'status' => $worldStatus,
                    'login_enabled' => $loginEnabled,
                    'game_host' => $host,
                    'game_port' => $port,
                ]);
                $world->save();
            });
        } catch (LogicException $exception) {
            $this->components->error($exception->getMessage());

            return self::FAILURE;
        } catch (Throwable) {
            $this->components->error('World Registry update failed.');

            return self::FAILURE;
        }

        $this->components->info(sprintf(
            'World Registry route %d is %s at %s:%d (login %s).',
            $id,
            $worldStatus->value,
            $host,
            $port,
            $loginEnabled ? 'enabled' : 'disabled',
        ));

        return self::SUCCESS;
    }

    private function positiveIntegerOption(string $name): ?int
    {
        $value = $this->stringOption($name);
        if ($value === null || preg_match('/^[1-9][0-9]*$/', $value) !== 1) {
            $this->components->error(sprintf('--%s must be a positive integer.', $name));

            return null;
        }

        return (int) $value;
    }

    private function booleanOption(string $name): ?bool
    {
        $value = $this->stringOption($name);
        if ($value === null) {
            return null;
        }

        return match (strtolower($value)) {
            '1', 'true', 'yes', 'on' => true,
            '0', 'false', 'no', 'off' => false,
            default => $this->invalidBooleanOption($name),
        };
    }

    private function stringOption(string $name): ?string
    {
        $value = $this->option($name);
        if (! is_string($value)) {
            $this->components->error(sprintf('--%s must be a string value.', $name));

            return null;
        }

        return trim($value);
    }

    private function invalidBooleanOption(string $name): null
    {
        $this->components->error(sprintf('--%s must be a boolean value.', $name));

        return null;
    }

    private function validHost(string $host): bool
    {
        if ($host === '' || mb_strlen($host) > 255 || str_contains($host, '://') || str_contains($host, '/')) {
            return false;
        }

        return filter_var($host, FILTER_VALIDATE_IP) !== false
            || filter_var($host, FILTER_VALIDATE_DOMAIN, FILTER_FLAG_HOSTNAME) !== false;
    }
}
