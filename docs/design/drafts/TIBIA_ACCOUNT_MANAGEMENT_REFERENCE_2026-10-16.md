# Tibia: audyt funkcji zalogowanego konta

**Status: DRAFT — materiał referencyjny; niezatwierdzone wymagania Oteryn.**

Powiązany zakres produktu: #277, #317, #319.

Data: 16 października 2026. Cel: rzeczywisty wzorzec zakresu i organizacji
Account Center Oteryn. To obserwacja Tibii, nie akceptacja jej reguł dla Oteryn.

Źródło: https://www.tibia.com/account/?subtopic=accountmanagement&page=overview
oraz oficjalny podręcznik Tibii, rozdział Account Management, otwarty z panelu.

## Zakres i wiarygodność

Przegląd obejmuje widoczne sekcje zalogowanego konta i dostępne bez zatwierdzania
operacji podformularze. Odczytano etykiety, pola, warunki oraz kroki formularzy.
Nie wpisywano haseł, recovery keys, TAN ani kodów produktów; nie zapisywano
ustawień, nie usuwano postaci, nie wysyłano wiadomości i nie składano zamówień.

Nie jest to test końcowego skutku każdej operacji. Niektóre stany wymagają
usuniętej postaci, zakupionego produktu, zmiany 2FA albo zgody na usługę. Ich
ograniczenia są jawnie wskazane poniżej; opis podręcznika nie jest dowodem
wykonania operacji na tym koncie. Nie kopiujemy danych osobowych ani transakcji.

## Inwentarz

| Obszar | Zaobserwowane funkcje | Głębokość przeglądu |
|---|---|---|
| Overview | status konta/Premium, pobranie klienta, zgłoszenia, lista postaci, Create Character, Manage Account | ekran konta |
| Lista postaci | nazwa, profesja, poziom, świat, oznaczenia ukrycia/nagrody, informacje o domu; Edit i Delete dla wybranej postaci | ekran konta; podręcznik dodatkowo wymienia gildię i warunkowe Undelete |
| Tworzenie postaci | nazwa/sugestie, płeć, świat, region, kategoria serwera i typ PvP, reCAPTCHA | formularz przed Submit |
| Edit postaci | dane nazwa/świat/płeć; Hide Account, komentarz, podpis forumowy | formularz przed Submit; dane postaci nie zostały uznane za edytowalne tylko dlatego, że są pokazane |
| Osiągnięcia | wybór maks.5 zdobytych osiągnięć na publiczny profil; Grade1–4 i Secret Achievements; opis dopuszcza zdobycze z innych postaci konta | formularz i opis, bez zmiany zaznaczeń |
| Delete | nazwa usuwanej postaci i potwierdzenie hasłem; komunikat o60dniach na przywrócenie i nieodwracalności po tym terminie | formularz przed Submit |
| Undelete | warunkowy odnośnik zamiast Delete i możliwość zachowania usuniętej postaci | potwierdzony opis oficjalny; nie usunięto postaci w celu otwarcia tego stanu |
| General Information | podstawowe dane konta, logowania, status, Coins, Loyalty, język emaili | ekran; rozwinięto listę języków bez wyboru/zapisu |
| Język emaili | português, deutsch, english, español, polski | rozwinięta lista |
| Change Password | Send Email, link potwierdzający ważny24h | pierwszy ekran; wiadomości nie wysłano |
| Manage Emails | odzyskiwanie przez Key/TAN, zmiana z okresem oczekiwania, secondary email | ekran i wszystkie4 formularze wymienione poniżej |
| Email przez Recovery Key | nowy email, segmenty klucza; dalszy proces wymaga ustawienia nowego hasła | formularz przed Submit |
| Email przez Recovery TAN | nowy email, TAN; opis TAN: jednorazowy i ważny24h | formularz przed Submit |
| Email z okresem oczekiwania | nowy email, hasło, opis finalizacji po30dniach i konieczności nowego hasła | formularz przed Submit |
| Secondary email | email, hasło, zalecenie innego dostawcy | formularz przed Submit |
| Security Wizard | ocena bezpieczeństwa, aktualność adresu/telefonu,2FA, ostatnie hasło, secondary email; skróty edycji | ekran; Continue wróciło do panelu |
| Terminate Account | potwierdzenie zakończenia konta i usunięcia wszystkich jego postaci, wymagane hasło | formularz przed Submit |
| Tibia Observer | połączenie urządzenia, Unlink, Reconnect App, opis funkcji aplikacji | widoczny stan połączenia i opis; nie odłączano ani nie odnawiano powiązania |
| Account Badges | wybór odznak pokazywanych przy publicznym wyszukiwaniu postaci | edytor,12 checkboxów na tym koncie, bez zapisu |
| Tell-A-Friend/Rewards | nagrody rekrutacyjne, Loyalty i Premium, nagrody zaproszonych | zakładka |
| Invite New Players | link polecający i Copy Link | zakładka; nie kopiowano/rozsyłano linku |
| Bring Back a Buddy | wybór zapraszającej postaci i nazwa postaci znajomego; opis wymogu12miesięcy nieaktywności | formularz przed Invite Friend |
| Referral Statistics | statystyki zaproszeń przez link, email i wcześniejszy system | zakładka |
| Loyalty Highscore Character | wybór reprezentującej postaci z nieukrytych; Submit; podręcznik opisuje fallback do najwyższego poziomu i brak widoczności, gdy wszystkie są ukryte | pole panelu i opis oficjalny; bez zmiany wyboru |
| Products Available | Coins, Premium, Recovery Key i Use Game Code | ekran oraz początki katalogów/realizacji |
| Premium/Coins Webshop | Select product → Enter payment data → Confirm your order → Summary; kraj, produkt, metoda płatności | pierwszy krok; nie wybrano płatności ani nie naciskano Next |
| Kategorie produktów | Coins transferable/initially non-transferable, Premium Time, Extra Services | katalog; w Extra Services zaobserwowano Recovery Key |
| Character Name Change | wymieniona jako produkt kupowany za Coins w Store | opis w panelu; nie jest polem Edit postaci; nie przetestowano zakupu/realizacji |
| Products Ready To Use | miejsce na posiadane produkty/usługi do wykorzystania | na koncie brak produktów; formularzy realizacji nie da się obejrzeć bez uprawnienia/zakupu |
| Use Game Code |4 części kodu; Enter Game Code → Verify your data → Result | pierwszy krok; nie podawano kodu |
| Gift Code | wybór postaci, segmenty kodu, Check Code i FAQ | pola panelu; bez kodu nie wykonano walidacji/realizacji |
| Token Exchange | zgody na usługę i Reown, portfel, import/eksport; wymagania2FA, rejestracji, conduct, gazuBNB; eksport wymaga postaci20+ | ekran wejściowy i warunki; nie akceptowano zgód ani nie podłączano portfela |
| Premium History | historia Premium | otwarty widok |
| Payments History | historia płatności, filtr i akcje Withdraw | otwarty widok; bez wykonywania wycofania |
| Coins History | historia, View szczegółów, odniesienia do postaci, paginacja | otwarty widok i szczegóły jednego wpisu przez View; bez kopiowania danych transakcji |
| Registration | dane adresowe i telefon; Edit/Remove | edytor: pola danych, potwierdzenie hasłem, link email i30dni; nie usuwano rejestracji |
| Authenticator App/Email Code | alternatywne metody2FA; na tym koncie aktywna aplikacja; przełączenie wymaga odłączenia | ekran konta i oficjalny opis etapów; nie zmieniano zabezpieczeń |
| Email Notifications | stan subskrypcji i Unsubscribe | ekran; bez zmiany subskrypcji |
| Tickets | przegląd/odpowiedź, temat/wiadomość, załącznik i nowe zgłoszenie | formularze; bez wysłania wiadomości/plików |
| New Ticket | General, Technical Problems, Payments, postać, temat, wiadomość | formularz przed Submit |
| Play Session Agreement | zgoda na rejestrowanie sesji gry | widoczny opis i Consent; bez wyrażenia zgody |
| Advertisement | zgoda na personalizowane reklamy zewnętrzne | widoczny opis i Consent; bez zmiany zgody |
| Trusted Devices | lista i Remove/Remove All; podręcznik opisuje30dni oraz usuwanie urządzeń po zmianie hasła/2FA | ekran i opis; bez cofania zaufania |
| Rule Violation Reports/Record | warunkowe sekcje naruszeń/regulaminu | wymienione w oficjalnej instrukcji; nie pojawiły się w obserwowanym panelu konta |

## Co z tego wynika dla Oteryn

1. Panel postaci musi zawierać konkretne dane i akcje, a nie wyłącznie nazwę
   i odnośnik do dwóch preferencji profilu.
2. Ustawienia profilu obejmują osobny zakres: powiązanie z kontem, komentarz,
   podpis, wybrane osiągnięcia. Główna postać/wybór rankingowy to jeszcze inny cel.
3. Usuwanie i przywracanie to rzeczywisty lifecycle z datą/granicą i potwierdzeniem,
   a nie kosmetyczny przycisk ukrywający rekord Platform.
4. Usługi (np.rename) są oddzielone od zwykłej edycji prezentacji; nie należy
   dopisywać darmowego rename z arbitralnym cooldownem na podstawie tego wzorca.
5. Bezpieczeństwo, powiadomienia, zgody, historia i produkty mają własne ekrany,
   stany, wymagania i skutki. Nie wolno traktować jednego przycisku jako ich zamiennika.
6. Zakres/zasady Oteryn wymagają sprawdzenia jego istniejących kontraktów; badanie
   referencji samo nie aktywuje płatności, transferów,2FA ani nowych mutacji Game.

## Granica tego przeglądu

Nie twierdzimy, że wykonano wszystkie operacje do końca. Ukończono odczyt
widocznej powierzchni oraz bezpiecznie dostępnych formularzy. Za bramkami
Submit/Send Email/Invite Friend/zgód/płatności/uprawnień pozostają dalsze stany,
których nie można empirycznie obejrzeć bez efektu na koncie. Transfer świata
nie został potwierdzony jako dostępny formularz tego konta WWW.


## Pochodzenie i granica publikacji

Właściciel polecił obejrzeć własną zalogowaną sesję jako przykład i następnie
zapisać obserwacje w repozytorium jako draft. Ten dokument zawiera wyłącznie
opis funkcji i zaobserwowanych warunków. Nie zawiera danych konta, nazw jego
postaci, adresów email, kwot/historii transakcji, tokenów ani materiałów
uwierzytelniających. Nie załączono zrzutów ekranu ani surowych logów sesji.

Wcześniejszy projekt darmowej zmiany nazwy z cooldownem24h i usunięcia z7dniami
na anulowanie został wycofany po uwadze właściciela. Nie jest częścią tego draftu
ani zaakceptowaną regułą Oteryn.60dni w tabeli to odczyt formularza Tibii.

Dalsza praca powinna zamienić obserwacje na macierz: istnieje w Oteryn / brak /
wymaga decyzji produktu / wymaga przyjętego kontraktu Game. Samo scalenie lub
zaakceptowanie tego raportu nie aktywuje żadnej z opisanych operacji.
