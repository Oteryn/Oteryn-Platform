<?php

return [
    'title' => 'Dzisiaj',
    'eyebrow' => 'Centrum informacji',
    'description' => 'Rzetelny publiczny obraz tego, co jest teraz ważne. Każda karta zachowuje własne źródło prawdy i stan dostępności.',
    'partial' => 'Część publicznych źródeł jest niedostępna. Dostępne karty pozostają widoczne bez wymyślania brakujących danych.',
    'unavailable' => 'Widok Dzisiaj nie może obecnie odczytać żadnego autorytatywnego publicznego źródła.',
    'cards_label' => 'Publiczne karty informacji Dzisiaj',
    'source' => 'Otwórz widok źródłowy',
    'actions' => [
        'view_event' => 'Zobacz wydarzenie',
        'read_news' => 'Czytaj aktualność',
    ],
    'badges' => [
        'featured' => 'Wyróżnione',
    ],
    'cards' => [
        'liveops' => [
            'eyebrow' => 'Świat na żywo',
            'title' => 'Sygnały świata',
            'unavailable' => 'Autorytatywne dane LiveOps nie są jeszcze udostępnione temu publicznemu widokowi. Stan runtime ani stan odzyskiwania nie są wnioskowane.',
            'empty' => 'Brak sygnału świata na żywo.',
            'empty_help' => 'Autorytatywne LiveOps nie zgłosiło odpowiedniego publicznego sygnału.',
            'partial' => 'Część danych świata jest nieaktualna, niedostępna, nieprawidłowa lub mieszana. Ostatnie znane fakty pozostają jednoznacznie oznaczone.',
            'states' => [
                'maintenance' => ['label' => 'Konserwacja', 'summary' => 'Dla tego świata skonfigurowano konserwację Platformy. Gotowość runtime nie może ominąć tej blokady.'],
                'policy_offline' => ['label' => 'Wyłączony konfiguracyjnie', 'summary' => 'Polityka Platformy oznacza ten świat jako niedostępny dla nowych wejść; nie jest to wniosek z awarii runtime.'],
                'login_disabled' => ['label' => 'Logowanie wyłączone', 'summary' => 'Polityka Platformy obecnie blokuje nowe wejścia do tego świata.'],
                'policy_unknown' => ['label' => 'Polityka nieznana', 'summary' => 'Platforma nie może obecnie potwierdzić dostępności nowych wejść.'],
                'ready' => ['label' => 'Gotowy', 'summary' => 'Świeże autorytatywne dane runtime potwierdzają gotowość wszystkich bieżących kanałów.'],
                'not_ready' => ['label' => 'Niegotowy', 'summary' => 'Świeże autorytatywne dane runtime wskazują, że bieżące kanały nie są gotowe.'],
                'degraded' => ['label' => 'Częściowo dostępny', 'summary' => 'Dane kanałów są mieszane; nie jest z tego wnioskowany stan offline całego świata.'],
                'stale' => ['label' => 'Nieaktualne dane', 'summary' => 'Ostatnie przyjęte dane runtime są nieaktualne i nie są traktowane jako bieżący stan online ani offline.'],
                'unavailable' => ['label' => 'Dane niedostępne', 'summary' => 'Brak bieżących autorytatywnych danych runtime; stan świata nie jest wymyślany.'],
                'invalid' => ['label' => 'Dane nieprawidłowe', 'summary' => 'Bieżące dane runtime nie przeszły walidacji autorytetu lub zastosowania i nie są używane jako stan świata.'],
            ],
        ],
        'announcements' => [
            'eyebrow' => 'Komunikaty',
            'title' => 'Ogłoszenia',
            'unavailable' => 'Opublikowane ogłoszenia są chwilowo niedostępne.',
            'empty' => 'Brak aktywnych ogłoszeń.',
            'empty_help' => 'Nie ma ogłoszenia w zatwierdzonym oknie publikacji.',
        ],
        'events' => [
            'eyebrow' => 'Kalendarz',
            'title' => 'Najbliższe wydarzenie',
            'unavailable' => 'Informacje o wydarzeniach są chwilowo niedostępne.',
            'empty' => 'Brak nadchodzącego wydarzenia.',
            'empty_help' => 'Nie ma zatwierdzonego nadchodzącego wydarzenia w tym języku.',
        ],
        'news' => [
            'eyebrow' => 'Kroniki',
            'title' => 'Najnowsze aktualności',
            'unavailable' => 'Opublikowane aktualności są chwilowo niedostępne.',
            'empty' => 'Brak opublikowanych aktualności.',
            'empty_help' => 'Nie ma skutecznie opublikowanych aktualności w tym języku.',
        ],
    ],
];
