<?php

namespace App\GameAuth\NativeLogin;

use RuntimeException;

/**
 * Internal: a concurrent request for the same attempt_ref committed first. The issuer rolls back
 * and re-reads the committed attempt once; this never reaches a client.
 *
 * @internal
 */
final class NativeAdmissionAttemptRace extends RuntimeException {}
