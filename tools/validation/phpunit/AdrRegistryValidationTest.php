<?php

namespace Tests\Unit\Architecture;

use PHPUnit\Framework\TestCase;
use Symfony\Component\Process\Process;

final class AdrRegistryValidationTest extends TestCase
{
    public function test_focused_validator_suite_passes(): void
    {
        $this->runPython('tools/validation/test_adr_registry.py');
    }

    public function test_repository_adr_registry_passes(): void
    {
        $this->runPython('tools/validation/adr_registry.py');
    }

    private function runPython(string $script): void
    {
        $root = dirname(__DIR__, 3);
        $process = new Process(['python3', $script], $root);
        $process->setTimeout(30);
        $process->run();

        $this->assertTrue(
            $process->isSuccessful(),
            trim($process->getOutput()."\n".$process->getErrorOutput())
        );
    }
}
