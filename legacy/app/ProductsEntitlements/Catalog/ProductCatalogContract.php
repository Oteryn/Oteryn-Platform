<?php

namespace App\ProductsEntitlements\Catalog;

final class ProductCatalogContract
{
    public const PROFILE_PLATFORM_ONLY = 'A';

    public const PROFILE_GAME_ACCOUNT = 'B';

    public const PROFILE_GAME_GRANT = 'C';

    public const PROFILE_CHARACTER_SERVICE = 'D';

    public const PROFILE_WALLET_PACKAGE = 'E';

    public const SCOPE_PLATFORM = 'platform';

    public const SCOPE_ACCOUNT = 'account';

    public const SCOPE_CHARACTER = 'character';

    public const SCOPE_WALLET = 'wallet';

    /** @return list<string> */
    public static function profiles(): array
    {
        return [
            self::PROFILE_PLATFORM_ONLY,
            self::PROFILE_GAME_ACCOUNT,
            self::PROFILE_GAME_GRANT,
            self::PROFILE_CHARACTER_SERVICE,
            self::PROFILE_WALLET_PACKAGE,
        ];
    }

    /** @return list<string> */
    public static function targetScopes(): array
    {
        return [
            self::SCOPE_PLATFORM,
            self::SCOPE_ACCOUNT,
            self::SCOPE_CHARACTER,
            self::SCOPE_WALLET,
        ];
    }
}
