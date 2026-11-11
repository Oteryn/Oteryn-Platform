<?php

namespace Tests\Unit\Architecture;

use PHPUnit\Framework\TestCase;
use Symfony\Component\Process\Process;

final class ArchitectureDecisionBacklogValidationTest extends TestCase
{
    public function test_focused_validator_suite_passes(): void
    {
        $this->runPython('tools/validation/test_architecture_decision_backlog.py');
    }

    public function test_repository_decision_backlog_passes(): void
    {
        $this->runPython('tools/validation/architecture_decision_backlog.py');
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
