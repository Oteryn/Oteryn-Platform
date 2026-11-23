-- Publish the initial editions only for slugs without an existing CMS page.
WITH editions(slug, en_title, en_body, pl_title, pl_body) AS (
    VALUES
    ('getting-started', 'Getting started', $en$
1. Read the [community rules](/en/rules) before joining.
2. [Create a portal account](/en/register), then open the verification link sent to your email address.
3. [Sign in](/en/login) and check the [server information](/en/server-information) before choosing a world. Look for client release announcements in [News](/en/news).

If you get stuck, check the [FAQ](/en/faq) or [contact player support](/en/support).
$en$, 'Pierwsze kroki', $pl$
1. Przeczytaj [zasady społeczności](/pl/rules), zanim dołączysz.
2. [Utwórz konto w portalu](/pl/register), a następnie otwórz link weryfikacyjny wysłany na Twój adres e-mail.
3. [Zaloguj się](/pl/login) i przed wyborem świata sprawdź [informacje o serwerach](/pl/server-information). Ogłoszeń o wydaniach klienta szukaj w [Aktualnościach](/pl/news).

Jeśli potrzebujesz pomocy, zajrzyj do [FAQ](/pl/faq) lub [skontaktuj się z pomocą dla graczy](/pl/support).
$pl$),
    ('rules', 'Community rules', $en$
## Treat other players with respect

Harassment, threats, hateful language, impersonation and sharing another person's private information without consent are prohibited. Avoid spam and use the appropriate channel for your message.

## Play fairly

Do not use bots or automation that plays on your behalf, exploit defects for an advantage, or help others evade moderation. Report reproducible defects privately using the [bug reporting guide](/en/support/report-a-bug).

## Protect your account

Keep your credentials and recovery codes private. Support will never ask you to send your password or a recovery code. Report suspected abuse or ask about a moderation decision through [player support](/en/support). See also the [Terms of Service](/en/legal/terms).
$en$, 'Zasady społeczności', $pl$
## Szanuj innych graczy

Nękanie, groźby, mowa nienawiści, podszywanie się pod innych i udostępnianie cudzych danych prywatnych bez zgody są zabronione. Unikaj spamu i wybieraj kanał odpowiedni do treści wiadomości.

## Graj uczciwie

Nie używaj botów ani automatyzacji grającej za Ciebie, nie wykorzystuj błędów dla uzyskania przewagi i nie pomagaj innym omijać decyzji moderacji. Błędy, które da się powtórzyć, zgłaszaj prywatnie zgodnie z [instrukcją zgłaszania błędów](/pl/support/report-a-bug).

## Chroń swoje konto

Zachowaj dane logowania i kody odzyskiwania dla siebie. Pomoc nigdy nie poprosi Cię o przesłanie hasła ani kodu odzyskiwania. Podejrzenie nadużycia lub pytanie o decyzję moderacji skieruj do [pomocy dla graczy](/pl/support). Zobacz też [Regulamin Oteryn](/pl/legal/terms).
$pl$),
    ('server-information', 'Server information', $en$
Use the [world list](/worlds) to check which worlds are listed and whether current status information is available. If the status is unknown, check again later. For planned maintenance and service announcements, see [News](/en/news).

If you are setting up for the first time, follow [Getting started](/en/start). If a connection problem persists, include the world, approximate time and exact error message in your [support ticket](/en/support).
$en$, 'Informacje o serwerach', $pl$
Na [liście światów](/worlds) sprawdzisz dostępne światy i dowiesz się, czy jest znany ich bieżący status. Jeśli status jest nieznany, sprawdź ponownie później. Informacje o planowanych przerwach i ogłoszenia znajdziesz w [Aktualnościach](/pl/news).

Jeśli konfigurujesz grę po raz pierwszy, zajrzyj do [Pierwszych kroków](/pl/start). Jeśli problem z połączeniem się utrzymuje, podaj w [zgłoszeniu do pomocy](/pl/support) świat, przybliżony czas i dokładny komunikat błędu.
$pl$),
    ('support', 'Player support', $en$
## Choose the right channel

- **Cannot sign in?** Use [account recovery](/en/forgot-password) to request a password reset.
- **Found a technical problem?** Follow the [bug reporting guide](/en/support/report-a-bug).
- **Need help with your account, player conduct or a moderation decision?** Sign in and [submit a support ticket](/en/support/tickets/create). Follow the conversation in [your tickets](/en/support/tickets).

Include the date and time, what you were trying to do, and any error message. Remove sensitive details from screenshots or logs. Never include your password or recovery codes.
$en$, 'Pomoc dla graczy', $pl$
## Wybierz właściwy kanał

- **Nie możesz się zalogować?** Skorzystaj z [odzyskiwania dostępu do konta](/pl/forgot-password), by poprosić o zresetowanie hasła.
- **Masz problem techniczny?** Skorzystaj z [instrukcji zgłaszania błędów](/pl/support/report-a-bug).
- **Potrzebujesz pomocy z kontem, zachowaniem gracza lub decyzją moderacji?** Zaloguj się i [wyślij zgłoszenie do pomocy](/pl/support/tickets/create). Odpowiedzi znajdziesz w [swoich zgłoszeniach](/pl/support/tickets).

Podaj datę i godzinę, opisz wykonywaną czynność i przepisz komunikat o błędzie. Usuń dane wrażliwe ze zrzutów ekranu i logów. Nigdy nie podawaj hasła ani kodów odzyskiwania.
$pl$),
    ('report-a-bug', 'Report a bug', $en$
[Open a support ticket](/en/support/tickets/create) and select the technical category. Describe the steps to reproduce the issue, what you expected, and what happened instead. Include the time and time zone, the affected page or world, and your browser or client version when relevant.

Copy the exact error message if one appeared. Remove personal information and account credentials from any material you share. If your report concerns another player or access to your account, see [player support](/en/support) for the appropriate channel.
$en$, 'Zgłoś błąd', $pl$
[Utwórz zgłoszenie do pomocy](/pl/support/tickets/create) i wybierz kategorię techniczną. Opisz kroki pozwalające odtworzyć problem, oczekiwany wynik i to, co rzeczywiście się stało. Podaj czas i strefę czasową, stronę lub świat, którego dotyczy błąd, oraz wersję przeglądarki lub klienta, jeśli ma to znaczenie.

Jeśli pojawił się komunikat o błędzie, przytocz go w całości. Usuń dane osobowe i dane logowania z przesyłanych materiałów. Jeśli sprawa dotyczy innego gracza lub dostępu do konta, wybierz właściwy kanał na stronie [pomocy dla graczy](/pl/support).
$pl$),
    ('faq-account-access', 'How do I recover access to my account?', $en$
Use [Forgot password](/en/forgot-password) with the email address associated with your account. Check your spam folder if no reset email arrives. If you use two-factor authentication and have lost your authenticator, use one of the recovery codes you saved when setting it up.
$en$, 'Jak odzyskać dostęp do konta?', $pl$
Użyj opcji [Nie pamiętam hasła](/pl/forgot-password) i podaj adres e-mail przypisany do konta. Jeśli wiadomość nie dotarła, sprawdź folder spamu. Jeśli korzystasz z uwierzytelniania dwuskładnikowego i nie masz już dostępu do aplikacji uwierzytelniającej, użyj jednego z kodów zapisanych podczas konfiguracji.
$pl$),
    ('faq-client-download', 'How will I know when the client is available?', $en$
Check [News](/en/news) for official client release announcements. If you find a download link elsewhere, check it against an official release announcement before installing anything. See [Getting started](/en/start) for account setup and server information.
$en$, 'Jak sprawdzę, czy klient gry jest dostępny?', $pl$
Szukaj oficjalnych ogłoszeń o wydaniach klienta w [Aktualnościach](/pl/news). Jeśli znajdziesz link do pobrania gdzie indziej, przed instalacją porównaj go z oficjalnym ogłoszeniem. W [Pierwszych krokach](/pl/start) znajdziesz instrukcję utworzenia konta i informacje o serwerach.
$pl$),
    ('faq-ticket-replies', 'Where can I read responses to my support request?', $en$
Sign in with the account that opened the request and select it in [your support tickets](/en/support/tickets). Reply in the existing conversation when adding information about the same issue.
$en$, 'Gdzie znajdę odpowiedź na moje zgłoszenie?', $pl$
Zaloguj się na konto, z którego wysłano zgłoszenie, i wybierz je na [liście swoich zgłoszeń](/pl/support/tickets). Jeśli chcesz uzupełnić informacje o tej samej sprawie, odpowiedz w istniejącej rozmowie.
$pl$)
),
new_pages AS (
    INSERT INTO pages (id, slug)
    SELECT uuidv7(), slug FROM editions
    ON CONFLICT (slug) DO NOTHING
    RETURNING id, slug
),
english AS (
    INSERT INTO editorial_pages (page_id, locale, title, body, published_at)
    SELECT p.id, 'en', e.en_title, trim(e.en_body, E'\n'), now()
    FROM new_pages p JOIN editions e USING (slug)
    RETURNING page_id, revision
)
INSERT INTO editorial_pages (page_id, locale, title, body, published_at, source_revision)
SELECT p.id, 'pl', e.pl_title, trim(e.pl_body, E'\n'), now(), en.revision
FROM new_pages p JOIN editions e USING (slug) JOIN english en ON en.page_id = p.id;
