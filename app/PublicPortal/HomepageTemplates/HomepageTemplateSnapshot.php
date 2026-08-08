<?php

namespace App\PublicPortal\HomepageTemplates;

final readonly class HomepageTemplateSnapshot
{
    public function __construct(
        public ?string $storedActiveKey,
        public string $activeKey,
        public ?string $previousKey,
        public int $version,
        public bool $drifted,
    ) {}

    public function canRollback(): bool
    {
        return $this->previousKey !== null && $this->previousKey !== $this->activeKey;
    }
}
