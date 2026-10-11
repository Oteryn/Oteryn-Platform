<?php

namespace App\CharacterAuthority;

/** Closed audit-only purpose set of `TransferCharacterOwnership` (§4.2). */
enum CharacterTransferPurpose: string
{
    case ListingEscrow = 'BAZAAR_LISTING_ESCROW';
    case Settlement = 'BAZAAR_SETTLEMENT';
    case EscrowReturn = 'BAZAAR_ESCROW_RETURN';
}
