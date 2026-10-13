<?php

namespace App\Console\Commands;

use App\GameAuth\NativeEvidence\NativeEvidenceHighWaterWitness;
use App\GameAuth\NativeEvidence\NativeEvidenceUnavailable;
use Illuminate\Console\Command;
use Throwable;

/**
 * Provenance-only init of the witness store (PostgreSQL P6a, design note D1): creates only the missing
 * half of the witness-store marker and database row. It advances no namespace floor, writes no .floor
 * file and deletes, renames or edits no witness entry. Every state other than a clean slate, a crash
 * between the two writes of a previous init, or an equal marker and row is refused with nothing changed.
 *
 * Output is one JSON line {"result", "store_id", "entries"}; the exit status is non-zero for `busy` and
 * for every refusal.
 */
final class InitNativeEvidenceProvenance extends Command
{
    protected $signature = 'game-auth:native-evidence:init-provenance';

    protected $description = 'Create the native evidence witness-store provenance (marker and database row) without advancing any floor.';

    public function handle(NativeEvidenceHighWaterWitness $witness): int
    {
        try {
            $result = $witness->initializeStoreProvenance();
        } catch (NativeEvidenceUnavailable $exception) {
            // Witness messages are fixed diagnostics.
            return $this->refuse($exception->getMessage());
        } catch (Throwable $exception) {
            return $this->refuse('Unexpected '.class_basename($exception).' during provenance init.');
        }

        $this->line((string) json_encode($result->toArray(), JSON_UNESCAPED_SLASHES));

        return $result->succeeded() ? self::SUCCESS : self::FAILURE;
    }

    private function refuse(string $reason): int
    {
        $this->components->error('Native evidence provenance init refused: '.$reason);

        return self::FAILURE;
    }
}
