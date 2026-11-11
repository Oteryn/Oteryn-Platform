<?php

namespace App\GameAuth\NativeAdmission;

use RuntimeException;

/**
 * The native admission issuer cannot sign (contract NATIVE_LOGIN_UNAVAILABLE).
 * Messages are internal diagnostics and never carry key material.
 */
final class NativeAdmissionUnavailable extends RuntimeException {}
