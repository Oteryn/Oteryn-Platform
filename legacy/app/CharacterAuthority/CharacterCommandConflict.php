<?php

namespace App\CharacterAuthority;

use RuntimeException;

/** The idempotency key is already bound to a different immutable intent. */
final class CharacterCommandConflict extends RuntimeException {}
