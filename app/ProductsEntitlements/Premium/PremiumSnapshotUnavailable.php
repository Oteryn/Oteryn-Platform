<?php

namespace App\ProductsEntitlements\Premium;

use RuntimeException;

/** Disabled, misconfigured or invalid durable state: the snapshot read answers 503. */
final class PremiumSnapshotUnavailable extends RuntimeException {}
