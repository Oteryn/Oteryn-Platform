<?php

namespace App\ProductsEntitlements\Store\Purchase;

/** Rolls back the Wallet savepoint, including insert-or-ignore, before storing a refusal. */
final class StoreInsufficientBalance extends \RuntimeException {}
