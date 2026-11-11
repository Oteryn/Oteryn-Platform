<?php

namespace App\CharacterProfiles;

use RuntimeException;

/** The native account-character projection cannot prove ownership right now. */
final class CharacterProfileUnavailable extends RuntimeException {}
