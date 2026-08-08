<?php

return [
    'admin' => [
        'title' => 'Szablony strony głównej',
        'eyebrow' => 'Prezentacja portalu',
        'heading' => 'Szablon strony głównej',
        'intro' => 'Podglądaj i aktywuj wyłącznie zatwierdzone w kodzie prezentacje strony głównej. Żądania publiczne nie mogą wybierać szablonu.',
        'current' => 'Aktualny szablon',
        'version' => 'Wersja :version',
        'active' => 'Aktywny',
        'preview' => 'Podgląd',
        'activate' => 'Aktywuj',
        'rollback' => 'Przywróć :template',
        'rollback_help' => 'Brak poprzedniego zatwierdzonego szablonu do przywrócenia.',
        'drift_warning' => 'Zapisany szablon strony głównej nie jest już zarejestrowany. Ruch publiczny bezpiecznie korzysta z domyślnego szablonu produkcyjnego do czasu wybrania zatwierdzonego szablonu.',
        'activated' => 'Szablon strony głównej został aktywowany.',
        'rolled_back' => 'Poprzedni szablon strony głównej został przywrócony.',
        'conflict' => 'Szablon strony głównej został zmieniony w innej sesji. Odśwież stronę i spróbuj ponownie.',
        'rollback_unavailable' => 'Brak zarejestrowanego poprzedniego szablonu do przywrócenia.',
        'dashboard_title' => 'Szablony strony głównej',
        'dashboard_help' => 'Podglądaj, aktywuj i przywracaj zatwierdzone prezentacje strony głównej.',
    ],
    'templates' => [
        'production' => [
            'label' => 'Produkcyjny',
            'description' => 'Aktualna strona główna Oteryn w stylistyce dark fantasy i deterministyczny domyślny widok publiczny.',
        ],
        'classic' => [
            'label' => 'Klasyczny portal',
            'description' => 'Wcześniejsza zweryfikowana prezentacja portalu, przywrócona wyłącznie za selektorem administratora.',
        ],
    ],
];
