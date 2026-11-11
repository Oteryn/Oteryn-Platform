<?php

namespace Tests\Support\Concurrency;

use Closure;
use Illuminate\Support\Facades\DB;
use JsonException;
use RuntimeException;
use Throwable;

/**
 * Runs the same closure in N separate OS processes that all start at the same instant.
 *
 * Each worker is a forked PHP process with its own database connection. Workers wait on a
 * file barrier so that they hit the database together, then write a JSON result file.
 */
final class ForkedWorkers
{
    /**
     * @param  Closure(int): array<string, mixed>  $work  Receives the worker index (0-based).
     * @return list<array<string, mixed>> Results ordered by worker index.
     */
    public static function run(int $count, string $connection, Closure $work): array
    {
        // Close the parent's socket before forking so that no child can terminate or reuse it.
        DB::purge($connection);

        $prefix = sys_get_temp_dir().'/oteryn-pg-race-'.bin2hex(random_bytes(8));
        $goPath = $prefix.'.go';
        $pids = [];

        try {
            for ($worker = 0; $worker < $count; $worker++) {
                $pid = pcntl_fork();
                if ($pid === -1) {
                    throw new RuntimeException('Unable to fork a concurrency worker.');
                }

                if ($pid === 0) {
                    self::child($prefix, $goPath, $worker, $work);
                }

                $pids[] = $pid;
            }

            $deadline = microtime(true) + 20;
            do {
                $ready = count(glob($prefix.'.*.ready') ?: []);
                if ($ready === $count) {
                    break;
                }
                usleep(10_000);
            } while (microtime(true) < $deadline);

            if ($ready !== $count) {
                throw new RuntimeException("Only {$ready} of {$count} workers reached the barrier.");
            }
            touch($goPath);

            foreach ($pids as $pid) {
                $status = 0;
                $waited = pcntl_waitpid($pid, $status);
                if ($waited !== $pid || ! is_int($status) || ! pcntl_wifexited($status) || pcntl_wexitstatus($status) !== 0) {
                    throw new RuntimeException('A concurrency worker exited abnormally.');
                }
            }

            $results = [];
            for ($worker = 0; $worker < $count; $worker++) {
                $contents = file_get_contents($prefix.'.'.$worker.'.json');
                if (! is_string($contents)) {
                    throw new RuntimeException("Worker {$worker} wrote no result.");
                }
                try {
                    $decoded = json_decode($contents, true, flags: JSON_THROW_ON_ERROR);
                } catch (JsonException $exception) {
                    throw new RuntimeException("Worker {$worker} wrote an invalid result.", 0, $exception);
                }
                if (! is_array($decoded)) {
                    throw new RuntimeException("Worker {$worker} wrote an invalid result.");
                }
                /** @var array<string, mixed> $decoded */
                $results[] = $decoded;
            }

            return $results;
        } finally {
            foreach (glob($prefix.'.*') ?: [] as $path) {
                @unlink($path);
            }
            DB::purge($connection);
        }
    }

    /** @param Closure(int): array<string, mixed> $work */
    private static function child(string $prefix, string $goPath, int $worker, Closure $work): never
    {
        touch($prefix.'.'.$worker.'.ready');
        $deadline = microtime(true) + 20;
        while (! is_file($goPath) && microtime(true) < $deadline) {
            usleep(1_000);
        }

        $result = ['outcome' => 'barrier_timeout'];
        if (is_file($goPath)) {
            try {
                $result = $work($worker);
            } catch (Throwable $exception) {
                $result = [
                    'outcome' => 'exception',
                    'class' => $exception::class,
                    'message' => $exception->getMessage(),
                ];
            }
        }

        file_put_contents($prefix.'.'.$worker.'.json', json_encode($result, JSON_THROW_ON_ERROR));
        exit(0);
    }
}
