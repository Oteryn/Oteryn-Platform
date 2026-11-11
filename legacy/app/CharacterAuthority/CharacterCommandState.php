<?php

namespace App\CharacterAuthority;

enum CharacterCommandState: string
{
    /** No receipt stored yet, whatever the intent's expiry: a timeout proves nothing (§2). */
    case Pending = 'PENDING';
    case Committed = 'COMMITTED';
    case Rejected = 'REJECTED';
}
