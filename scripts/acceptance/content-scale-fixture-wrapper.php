<?php

declare(strict_types=1);

/**
 * Execute one content-scale fixture in an isolated PHP process and expose only
 * its final JSON object on stdout. Framework informational output remains
 * diagnostic-only and cannot corrupt the Playwright fixture contract.
 */
function runContentScaleFixture(string $implementation): never
{
    if (! is_file($implementation)) {
        fwrite(STDERR, "Missing content-scale fixture implementation: {$implementation}\n");
        exit(1);
    }

    $command = [PHP_BINARY, $implementation];
    $descriptorSpec = [
        0 => ['pipe', 'r'],
        1 => ['pipe', 'w'],
        2 => ['pipe', 'w'],
    ];
    $process = proc_open($command, $descriptorSpec, $pipes, dirname(__DIR__, 2));

    if (! is_resource($process)) {
        fwrite(STDERR, "Cannot start content-scale fixture implementation.\n");
        exit(1);
    }

    fclose($pipes[0]);
    $stdout = stream_get_contents($pipes[1]);
    $stderr = stream_get_contents($pipes[2]);
    fclose($pipes[1]);
    fclose($pipes[2]);
    $status = proc_close($process);

    if ($status !== 0) {
        fwrite(STDERR, $stdout);
        fwrite(STDERR, $stderr);
        exit($status);
    }

    $lines = preg_split('/\R/u', $stdout) ?: [];
    for ($index = count($lines) - 1; $index >= 0; $index--) {
        $candidate = trim($lines[$index]);
        if ($candidate === '') {
            continue;
        }

        try {
            $decoded = json_decode($candidate, true, 512, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            continue;
        }

        if (! is_array($decoded)) {
            continue;
        }

        fwrite(STDOUT, $candidate.PHP_EOL);
        exit(0);
    }

    fwrite(STDERR, "Content-scale fixture did not emit a final JSON object.\n");
    fwrite(STDERR, $stdout);
    fwrite(STDERR, $stderr);
    exit(1);
}
