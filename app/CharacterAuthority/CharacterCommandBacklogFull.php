<?php

namespace App\CharacterAuthority;

/** `CHARCMD-PENDING-BACKLOG` is reached; the caller shows a retry-later state (§4.1). */
final class CharacterCommandBacklogFull extends CharacterCommandUnavailable {}
