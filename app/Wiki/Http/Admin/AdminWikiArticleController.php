<?php

namespace App\Wiki\Http\Admin;

use App\Admin\AdminAuthorization;
use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use App\Wiki\Application\Rendering\WikiMarkdownRenderer;
use App\Wiki\Application\WikiAdminArticleWriter;
use App\Wiki\Application\WikiArticleService;
use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Domain\WikiLocale;
use App\Wiki\Domain\WikiTranslationInput;
use App\Wiki\Http\Admin\Requests\AdminWikiArticleRequest;
use App\Wiki\Http\Admin\Requests\AdminWikiRestoreRequest;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use App\Wiki\Infrastructure\Models\WikiRevision;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Database\Query\Builder;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Collection;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\URL;
use Illuminate\Validation\Rule;
use Illuminate\Validation\ValidationException;
use InvalidArgumentException;
use RuntimeException;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

final readonly class AdminWikiArticleController
{
    public function __construct(private AdminAuthorization $authorization) {}

    public function index(Request $request): View
    {
        $filters = $request->validate([
            'status' => ['nullable', 'string', Rule::in(array_map(
                static fn (WikiArticleStatus $status): string => $status->value,
                WikiArticleStatus::cases(),
            ))],
            'locale' => ['nullable', 'string', Rule::in(WikiLocale::values())],
            'category_id' => ['nullable', 'integer', 'exists:wiki_categories,id'],
        ]);
        $status = $this->nullableValidatedString($filters, 'status');
        $locale = $this->nullableValidatedString($filters, 'locale');
        $categoryId = $this->nullableValidatedInteger($filters, 'category_id');

        $query = DB::table('wiki_articles as a')
            ->leftJoin('wiki_article_translations as en', function (JoinClause $join): void {
                $join->on('en.article_id', '=', 'a.id')->where('en.locale', 'en');
            })
            ->leftJoin('wiki_article_translations as pl', function (JoinClause $join): void {
                $join->on('pl.article_id', '=', 'a.id')->where('pl.locale', 'pl');
            })
            ->select([
                'a.id',
                'a.content_type',
                'a.status',
                'a.is_featured',
                'a.sort_order',
                'a.lock_version',
                'a.published_at',
                'a.updated_at',
                'en.title as en_title',
                'pl.title as pl_title',
            ]);

        if ($status !== null) {
            $query->where('a.status', $status);
        }

        if ($locale !== null) {
            $query->whereExists(function (Builder $translations) use ($locale): void {
                $translations->from('wiki_article_translations as filter_translation')
                    ->whereColumn('filter_translation.article_id', 'a.id')
                    ->where('filter_translation.locale', $locale);
            });
        }

        if ($categoryId !== null) {
            $query->whereExists(function (Builder $categories) use ($categoryId): void {
                $categories->from('wiki_article_category as filter_category')
                    ->whereColumn('filter_category.article_id', 'a.id')
                    ->where('filter_category.category_id', $categoryId);
            });
        }

        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return view('admin.wiki.articles.index', [
            'articles' => $query
                ->orderByDesc('a.updated_at')
                ->orderByDesc('a.id')
                ->paginate(25)
                ->withQueryString(),
            'statuses' => WikiArticleStatus::cases(),
            'categories' => $this->categoryOptions(),
            'filters' => [
                'status' => $status,
                'locale' => $locale,
                'category_id' => $categoryId,
            ],
            'canManageArticles' => $this->authorization->allows($identity, AdminPermission::MANAGE_WIKI_ARTICLES),
            'canPublish' => $this->authorization->allows($identity, AdminPermission::PUBLISH_WIKI),
        ]);
    }

    public function create(Request $request): View
    {
        return $this->form($request, null);
    }

    public function store(
        AdminWikiArticleRequest $request,
        WikiAdminArticleWriter $writer,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $article = $writer->create(
                $identity,
                $request->string('content_type')->toString(),
                $this->translations($request),
                $this->changeNote($request),
                $request->boolean('is_featured'),
                $request->integer('sort_order'),
                $this->categoryIds($request),
            );
        } catch (InvalidArgumentException $exception) {
            throw ValidationException::withMessages(['wiki' => $exception->getMessage()]);
        }

        return redirect()
            ->route('admin.wiki.articles.edit', $article)
            ->with('status', 'Wiki article draft created.');
    }

    public function edit(Request $request, WikiArticle $article): View
    {
        return $this->form($request, $article);
    }

    public function update(
        AdminWikiArticleRequest $request,
        WikiArticle $article,
        WikiAdminArticleWriter $writer,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $saved = $writer->update(
                $identity,
                $article,
                $request->integer('lock_version'),
                $request->string('content_type')->toString(),
                $this->translations($request),
                $this->changeNote($request),
                $request->boolean('is_featured'),
                $request->integer('sort_order'),
                $this->categoryIds($request),
            );
        } catch (InvalidArgumentException $exception) {
            throw ValidationException::withMessages(['wiki' => $exception->getMessage()]);
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()
            ->route('admin.wiki.articles.edit', $saved)
            ->with('status', 'Wiki article draft saved.');
    }

    public function preview(
        WikiArticle $article,
        string $locale,
        WikiMarkdownRenderer $renderer,
    ): View {
        if (WikiLocale::tryFrom($locale) === null) {
            abort(404);
        }

        $translation = WikiArticleTranslation::query()
            ->where('article_id', $article->id)
            ->where('locale', $locale)
            ->first();
        abort_unless($translation instanceof WikiArticleTranslation, 404);

        return view('admin.wiki.articles.preview', [
            'article' => $article,
            'translation' => $translation,
            'rendered' => $renderer->render($translation->source_markdown),
        ]);
    }

    public function revisions(Request $request, WikiArticle $article): View
    {
        $validated = $request->validate([
            'locale' => ['nullable', 'string', Rule::in(WikiLocale::values())],
        ]);
        $locale = $this->nullableValidatedString($validated, 'locale');
        $query = WikiRevision::query()->where('article_id', $article->id);

        if ($locale !== null) {
            $query->where('locale', $locale);
        }

        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return view('admin.wiki.articles.revisions', [
            'article' => $article,
            'translations' => WikiArticleTranslation::query()
                ->where('article_id', $article->id)
                ->get()
                ->keyBy('locale'),
            'revisions' => $query
                ->orderByDesc('created_at')
                ->orderByDesc('id')
                ->paginate(50)
                ->withQueryString(),
            'locale' => $locale,
            'canPublish' => $this->authorization->allows($identity, AdminPermission::PUBLISH_WIKI),
        ]);
    }

    public function restore(
        AdminWikiRestoreRequest $request,
        WikiArticle $article,
        WikiRevision $revision,
        WikiArticleService $articles,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $saved = $articles->restoreRevision(
                $identity,
                $article,
                $request->integer('lock_version'),
                $revision,
                $this->changeNote($request),
            );
        } catch (InvalidArgumentException $exception) {
            throw ValidationException::withMessages(['revision' => $exception->getMessage()]);
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()
            ->route('admin.wiki.articles.revisions', $saved)
            ->with('status', 'Historical Wiki revision restored as a new revision.');
    }

    private function form(Request $request, ?WikiArticle $article): View
    {
        $translations = $article === null
            ? collect()
            : WikiArticleTranslation::query()
                ->where('article_id', $article->id)
                ->get()
                ->keyBy('locale');
        $selectedCategoryIds = $article === null
            ? []
            : DB::table('wiki_article_category')
                ->where('article_id', $article->id)
                ->orderBy('sort_order')
                ->pluck('category_id')
                ->map(fn (mixed $id): int => $this->integer($id, 'Wiki category'))
                ->all();
        $previewUrls = [];

        if ($article !== null) {
            foreach (WikiLocale::values() as $locale) {
                if ($translations->has($locale)) {
                    $previewUrls[$locale] = URL::temporarySignedRoute(
                        'admin.wiki.articles.preview',
                        now()->addMinutes(10),
                        ['article' => $article, 'locale' => $locale],
                    );
                }
            }
        }

        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return view('admin.wiki.articles.form', [
            'article' => $article,
            'translations' => $translations,
            'categories' => $this->categoryOptions(),
            'selectedCategoryIds' => $selectedCategoryIds,
            'previewUrls' => $previewUrls,
            'canPublish' => $this->authorization->allows($identity, AdminPermission::PUBLISH_WIKI),
        ]);
    }

    /** @return list<WikiTranslationInput> */
    private function translations(AdminWikiArticleRequest $request): array
    {
        $validated = $request->validated();
        $rawTranslations = $validated['translations'] ?? [];

        if (! is_array($rawTranslations)) {
            throw ValidationException::withMessages([
                'translations' => 'Wiki translations must be an object.',
            ]);
        }

        $translations = [];

        foreach (WikiLocale::values() as $locale) {
            $raw = $rawTranslations[$locale] ?? null;

            if (! is_array($raw)) {
                continue;
            }

            $title = $this->translationString($raw, $locale, 'title');
            $slug = $this->translationString($raw, $locale, 'slug');
            $summary = $this->translationString($raw, $locale, 'summary');
            $sourceMarkdown = $this->translationString($raw, $locale, 'source_markdown');

            if ($locale === 'pl' && $title === '' && $slug === '' && $summary === '' && $sourceMarkdown === '') {
                continue;
            }

            try {
                $translations[] = new WikiTranslationInput(
                    $locale,
                    $title,
                    $slug,
                    $summary,
                    $sourceMarkdown,
                );
            } catch (InvalidArgumentException $exception) {
                throw ValidationException::withMessages([
                    "translations.{$locale}" => $exception->getMessage(),
                ]);
            }
        }

        if ($translations === [] || $translations[0]->locale !== 'en') {
            throw ValidationException::withMessages([
                'translations.en' => 'An English Wiki translation is required.',
            ]);
        }

        return $translations;
    }

    /** @param array<array-key, mixed> $translation */
    private function translationString(array $translation, string $locale, string $field): string
    {
        $value = $translation[$field] ?? '';

        if ($value === null) {
            return '';
        }

        if (! is_string($value)) {
            throw ValidationException::withMessages([
                "translations.{$locale}.{$field}" => 'Wiki translation fields must be strings.',
            ]);
        }

        return trim($value);
    }

    /** @return list<int> */
    private function categoryIds(AdminWikiArticleRequest $request): array
    {
        $validated = $request->validated();
        $values = $validated['category_ids'] ?? [];

        if ($values === null) {
            return [];
        }

        if (! is_array($values)) {
            throw ValidationException::withMessages([
                'category_ids' => 'Wiki categories must be a list.',
            ]);
        }

        return array_values(array_map(
            fn (mixed $value): int => $this->integer($value, 'Wiki category'),
            $values,
        ));
    }

    private function changeNote(Request $request): ?string
    {
        $value = trim($request->string('change_note')->toString());

        return $value === '' ? null : $value;
    }

    /** @return Collection<int, object> */
    private function categoryOptions(): Collection
    {
        return DB::table('wiki_categories as c')
            ->leftJoin('wiki_category_translations as en', function (JoinClause $join): void {
                $join->on('en.category_id', '=', 'c.id')->where('en.locale', 'en');
            })
            ->leftJoin('wiki_category_translations as pl', function (JoinClause $join): void {
                $join->on('pl.category_id', '=', 'c.id')->where('pl.locale', 'pl');
            })
            ->select([
                'c.id',
                'c.parent_id',
                'c.key',
                'c.visible',
                'c.sort_order',
                'en.name as en_name',
                'pl.name as pl_name',
            ])
            ->orderBy('c.sort_order')
            ->orderBy('c.key')
            ->orderBy('c.id')
            ->get();
    }

    /** @param array<string, mixed> $validated */
    private function nullableValidatedString(array $validated, string $key): ?string
    {
        $value = $validated[$key] ?? null;

        return is_string($value) && $value !== '' ? $value : null;
    }

    /** @param array<string, mixed> $validated */
    private function nullableValidatedInteger(array $validated, string $key): ?int
    {
        $value = $validated[$key] ?? null;

        return $value === null ? null : $this->integer($value, $key);
    }

    private function integer(mixed $value, string $description): int
    {
        if (is_int($value)) {
            return $value;
        }

        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }

        throw new RuntimeException("Expected an integer-compatible {$description} identifier.");
    }
}
