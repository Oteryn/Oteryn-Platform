<?php

return [
    'updater' => [
        'title' => 'Stan automatycznego aktualizatora',
        'trust_notice' => 'Ta strona pokazuje wyłącznie uzgodniony przez Platformę publiczny stan metadanych TUF. Klient Oteryn niezależnie weryfikuje podpisy TUF, świeżość i zaufane metadane plików; Download Center nie jest źródłem zaufania.',
        'policy' => 'Rewizja polityki :revision · aktualizacja :mode · minimalna obsługiwana sekwencja :minimum · metadane wygasają :expires UTC.',
        'states' => [
            'browser_only' => 'Plik jest publikowany tylko do pobrania w przeglądarce. To wydanie nie ma tożsamości automatycznego aktualizatora.',
            'pending' => 'Tożsamość aktualizatora istnieje, ale żadna aktywna w Platformie podpisana generacja nie wskazuje tego wydania przeglądarkowego.',
            'active' => 'Aktywna w Platformie podpisana generacja wskazuje dokładnie to wydanie. Nadal wymagana jest niezależna weryfikacja TUF po stronie klienta.',
            'browser_mismatch' => 'Bieżące wydanie przeglądarkowe i bieżące wydanie aktualizatora są różne. Pobieranie z przeglądarki pozostaje dostępne, ale stan nie jest po cichu uznawany za stan aktualizatora.',
            'withdrawn' => 'To wydanie aktualizatora zostało wycofane z nowych polityk. Publikacja przeglądarkowa pozostaje odrębnym stanem.',
            'revoked' => 'Aktywna podpisana polityka unieważnia tę tożsamość wydania dla automatycznej aktualizacji.',
            'target_revoked' => 'Aktywna podpisana polityka unieważnia co najmniej jeden dokładny cel platforma/architektura w tym bieżącym wydaniu.',
            'trust_expired' => 'Ostatnie aktywne metadane aktualizatora wygasły. Automatyczna aktualizacja pozostaje niedostępna do czasu uzgodnienia świeżego zaufanego stanu.',
            'degraded' => 'Stan aktualizatora jest niejednoznaczny lub niedostępny i dlatego jest prezentowany fail-closed. Publikacja przeglądarkowa pozostaje odrębna.',
        ],
    ],
];