<?php

namespace App\PublicGameData;

use RuntimeException;

/**
 * The configured public game-data source is switched off or not connected.
 */
final class PublicGameDataUnavailable extends RuntimeException {}
