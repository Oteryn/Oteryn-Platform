<?php

namespace App\CharacterAuthority;

use RuntimeException;

/** Commands are disabled, misconfigured or their stored state is unusable; nothing was written. */
class CharacterCommandUnavailable extends RuntimeException {}
