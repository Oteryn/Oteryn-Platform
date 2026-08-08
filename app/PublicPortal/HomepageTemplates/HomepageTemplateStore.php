<?php

namespace App\PublicPortal\HomepageTemplates;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use RuntimeException;
use stdClass;

final readonly class HomepageTemplateStore
{
    private const SETTING_ID = 1;

    public function __construct(
        private HomepageTemplateRegistry $registry,
        private AdminAuditRecorder $audit,
    ) {}

    public function snapshot(): HomepageTemplateSnapshot
    {
        $record = DB::table('homepage_template_settings')
            ->where('id', self::SETTING_ID)
            ->first();

        return $this->snapshotFromRecord($record instanceof stdClass ? $record : null);
    }

    public function activate(Identity $actor, string $templateKey, int $expectedVersion): HomepageTemplateSnapshot
    {
        if (! $this->registry->has($templateKey)) {
            throw new InvalidArgumentException('Homepage template key is not registered.');
        }

        return DB::transaction(function () use ($actor, $templateKey, $expectedVersion): HomepageTemplateSnapshot {
            $record = $this->lockedRecord();
            $version = $this->versionFromRecord($record);

            if ($version !== $expectedVersion) {
                throw new HomepageTemplateConflict('Homepage template version is stale.');
            }

            $storedActiveKey = is_string($record->active_key) ? $record->active_key : null;
            $currentKey = $this->registry->resolve($storedActiveKey);

            if ($storedActiveKey === $templateKey) {
                $this->audit->record(
                    $actor->id,
                    'portal.homepage_template.activate',
                    'homepage_template_setting',
                    (string) self::SETTING_ID,
                    [
                        'from' => $currentKey,
                        'to' => $templateKey,
                        'version' => $version,
                        'changed' => false,
                    ],
                );

                return $this->snapshotFromRecord($record);
            }

            $newVersion = $version + 1;

            DB::table('homepage_template_settings')
                ->where('id', self::SETTING_ID)
                ->update([
                    'active_key' => $templateKey,
                    'previous_key' => $currentKey,
                    'version' => $newVersion,
                    'updated_by_identity_id' => $actor->id,
                    'updated_at' => now(),
                ]);

            $this->audit->record(
                $actor->id,
                'portal.homepage_template.activate',
                'homepage_template_setting',
                (string) self::SETTING_ID,
                [
                    'from' => $currentKey,
                    'to' => $templateKey,
                    'version' => $newVersion,
                    'changed' => true,
                ],
            );

            return new HomepageTemplateSnapshot(
                storedActiveKey: $templateKey,
                activeKey: $templateKey,
                previousKey: $currentKey,
                version: $newVersion,
                drifted: false,
            );
        }, 3);
    }

    public function rollback(Identity $actor, int $expectedVersion): HomepageTemplateSnapshot
    {
        return DB::transaction(function () use ($actor, $expectedVersion): HomepageTemplateSnapshot {
            $record = $this->lockedRecord();
            $version = $this->versionFromRecord($record);

            if ($version !== $expectedVersion) {
                throw new HomepageTemplateConflict('Homepage template version is stale.');
            }

            $storedActiveKey = is_string($record->active_key) ? $record->active_key : null;
            $currentKey = $this->registry->resolve($storedActiveKey);
            $previousKey = is_string($record->previous_key) && $this->registry->has($record->previous_key)
                ? $record->previous_key
                : null;

            if ($previousKey === null || $previousKey === $currentKey) {
                throw new HomepageTemplateRollbackUnavailable('No registered previous homepage template is available.');
            }

            $newVersion = $version + 1;

            DB::table('homepage_template_settings')
                ->where('id', self::SETTING_ID)
                ->update([
                    'active_key' => $previousKey,
                    'previous_key' => $currentKey,
                    'version' => $newVersion,
                    'updated_by_identity_id' => $actor->id,
                    'updated_at' => now(),
                ]);

            $this->audit->record(
                $actor->id,
                'portal.homepage_template.rollback',
                'homepage_template_setting',
                (string) self::SETTING_ID,
                [
                    'from' => $currentKey,
                    'to' => $previousKey,
                    'version' => $newVersion,
                ],
            );

            return new HomepageTemplateSnapshot(
                storedActiveKey: $previousKey,
                activeKey: $previousKey,
                previousKey: $currentKey,
                version: $newVersion,
                drifted: false,
            );
        }, 3);
    }

    private function lockedRecord(): stdClass
    {
        $record = DB::table('homepage_template_settings')
            ->where('id', self::SETTING_ID)
            ->lockForUpdate()
            ->first();

        if ($record instanceof stdClass) {
            return $record;
        }

        $now = now();

        DB::table('homepage_template_settings')->insertOrIgnore([
            'id' => self::SETTING_ID,
            'active_key' => HomepageTemplateRegistry::DEFAULT_KEY,
            'previous_key' => null,
            'version' => 0,
            'updated_by_identity_id' => null,
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        $record = DB::table('homepage_template_settings')
            ->where('id', self::SETTING_ID)
            ->lockForUpdate()
            ->first();

        if (! $record instanceof stdClass) {
            throw new RuntimeException('Homepage template setting could not be initialized.');
        }

        return $record;
    }

    private function snapshotFromRecord(?stdClass $record): HomepageTemplateSnapshot
    {
        if ($record === null) {
            return new HomepageTemplateSnapshot(
                storedActiveKey: null,
                activeKey: HomepageTemplateRegistry::DEFAULT_KEY,
                previousKey: null,
                version: 0,
                drifted: false,
            );
        }

        $storedActiveKey = is_string($record->active_key) ? $record->active_key : null;
        $activeKey = $this->registry->resolve($storedActiveKey);
        $previousKey = is_string($record->previous_key) && $this->registry->has($record->previous_key)
            ? $record->previous_key
            : null;

        return new HomepageTemplateSnapshot(
            storedActiveKey: $storedActiveKey,
            activeKey: $activeKey,
            previousKey: $previousKey,
            version: $this->versionFromRecord($record),
            drifted: $storedActiveKey !== null && $storedActiveKey !== $activeKey,
        );
    }

    private function versionFromRecord(stdClass $record): int
    {
        $version = $record->version ?? null;

        if (is_int($version)) {
            return $version;
        }

        if (is_string($version) && ctype_digit($version)) {
            return (int) $version;
        }

        throw new RuntimeException('Homepage template setting version is invalid.');
    }
}
