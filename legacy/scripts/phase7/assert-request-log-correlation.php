<?php

declare(strict_types=1);

if ($argc !== 5) {
    fwrite(STDERR, "Usage: php assert-request-log-correlation.php <log-file> <request-id> <method> <status>\n");
    exit(2);
}

[, $logFile, $requestId, $expectedMethod, $expectedStatusRaw] = $argv;

if (! is_file($logFile) || ! is_readable($logFile)) {
    fwrite(STDERR, "Structured request log is unavailable.\n");
    exit(1);
}

if (preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i', $requestId) !== 1) {
    fwrite(STDERR, "Response request ID is not a valid UUID.\n");
    exit(1);
}

$expectedStatus = filter_var($expectedStatusRaw, FILTER_VALIDATE_INT);

if ($expectedStatus === false || $expectedStatus < 100 || $expectedStatus > 599) {
    fwrite(STDERR, "Expected HTTP status is invalid.\n");
    exit(2);
}

$matches = [];
$handle = fopen($logFile, 'rb');

if ($handle === false) {
    fwrite(STDERR, "Structured request log could not be opened.\n");
    exit(1);
}

try {
    while (($line = fgets($handle)) !== false) {
        $entry = json_decode(trim($line), true);

        if (! is_array($entry) || ($entry['message'] ?? null) !== 'http.request.completed') {
            continue;
        }

        $context = $entry['context'] ?? null;

        if (! is_array($context) || ($context['request_id'] ?? null) !== $requestId) {
            continue;
        }

        $matches[] = $context;
    }
} finally {
    fclose($handle);
}

if (count($matches) !== 1) {
    fwrite(STDERR, sprintf(
        "Expected exactly one correlated request-completion event; found %d.\n",
        count($matches),
    ));
    exit(1);
}

$context = $matches[0];

if (($context['method'] ?? null) !== $expectedMethod) {
    fwrite(STDERR, "Correlated request method does not match the HTTP probe.\n");
    exit(1);
}

if (($context['status'] ?? null) !== $expectedStatus) {
    fwrite(STDERR, "Correlated request status does not match the HTTP probe.\n");
    exit(1);
}

fwrite(STDOUT, "Correlated response request ID with exactly one structured request-completion event.\n");
