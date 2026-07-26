<?php

namespace App\Wiki\Http\Admin;

use App\Identity\Models\Identity;
use App\Wiki\Application\WikiAdminCategoryWriter;
use App\Wiki\Domain\WikiCategoryTranslationInput;
use App\Wiki\Domain\WikiLocale;
use App\Wiki\Http\Admin\Requests\AdminWikiCategoryRequest;
use App\Wiki\Infrastructure\Models\WikiCategory;
use App\Wiki\Infrastructure\Models\WikiCategoryTranslation;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Http\RedirectResponse;
use Illuminate\Support\Collection;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;
use InvalidArgumentException;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

final class AdminWikiCategoryController
{
    public function index(): View
    {
        return view('admin.wiki.categories.index', [
            'categories' => $this->categoryRows(),
        ]);
    }

    public function create(): View
    {
        return $this->form(null);
    }

    public function store(
        AdminWikiCategoryRequest $request,
        WikiAdminCategoryWriter $writer,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $category = $writer->create(
                $identity,
                $request->string('key')->toString(),
                $this->translations($request),
                $this->parent($request),
                $request->integer('sort_order'),
                $request->boolean('visible'),
            );
        } catch (InvalidArgumentException $exception) {
            throw ValidationException::withMessages(['wiki' => $exception->getMessage()]);
        }

        return redirect()
            ->route('admin.wiki.categories.edit', $category)
            ->with('status', 'Wiki category created.');
    }

    public function edit(WikiCategory $category): View
    {
        return $this->form($category);
    }

    public function update(
        AdminWikiCategoryRequest $request,
        WikiCategory $category,
        WikiAdminCategoryWriter $writer,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $saved = $writer->update(
                $identity,
                $category,
                $request->integer('lock_version'),
                $request->string('key')->toString(),
                $this->translations($request),
                $this->parent($request),
                $request->integer('sort_order'),
                $request->boolean('visible'),
            );
        } catch (InvalidArgumentException $exception) {
            throw ValidationException::withMessages(['wiki' => $exception->getMessage()]);
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()
            ->route('admin.wiki.categories.edit', $saved)
            ->with('status', 'Wiki category saved.');
    }

    private function form(?WikiCategory $category): View
    {
        return view('admin.wiki.categories.form', [
            'category' => $category,
            'translations' => $category === null
                ? collect()
                : WikiCategoryTranslation::query()
                    ->where('category_id', $category->id)
                    ->get()
                    ->keyBy('locale'),
            'parentOptions' => $this->categoryRows()
                ->reject(static fn (object $row): bool => $category !== null && (int) $row->id === $category->id)
                ->values(),
        ]);
    }

    /** @return list<WikiCategoryTranslationInput> */
    private function translations(AdminWikiCategoryRequest $request): array
    {
        $validated = $request->validated();
        $rawTranslations = $validated['translations'] ?? [];

        if (! is_array($rawTranslations)) {
            throw ValidationException::withMessages([
                'translations' => 'Wiki category translations must be an object.',
            ]);
        }

        $translations = [];

        foreach (WikiLocale::values() as $locale) {
            $raw = $rawTranslations[$locale] ?? null;

            if (! is_array($raw)) {
                continue;
            }

            $name = $this->translationString($raw, $locale, 'name');
            $slug = $this->translationString($raw, $locale, 'slug');
            $description = $this->translationString($raw, $locale, 'description');

            if ($locale === 'pl' && $name === '' && $slug === '' && $description === '') {
                continue;
            }

            try {
                $translations[] = new WikiCategoryTranslationInput(
                    $locale,
                    $name,
                    $slug,
                    $description === '' ? null : $description,
                );
            } catch (InvalidArgumentException $exception) {
                throw ValidationException::withMessages([
                    "translations.{$locale}" => $exception->getMessage(),
                ]);
            }
        }

        if ($translations === [] || $translations[0]->locale !== 'en') {
            throw ValidationException::withMessages([
                'translations.en' => 'An English Wiki category translation is required.',
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
                "translations.{$locale}.{$field}" => 'Wiki category translation fields must be strings.',
            ]);
        }

        return trim($value);
    }

    private function parent(AdminWikiCategoryRequest $request): ?WikiCategory
    {
        if (! $request->filled('parent_id')) {
            return null;
        }

        return WikiCategory::query()->findOrFail($request->integer('parent_id'));
    }

    /** @return Collection<int, object> */
    private function categoryRows(): Collection
    {
        return DB::table('wiki_categories as c')
            ->leftJoin('wiki_category_translations as en', function (JoinClause $join): void {
                $join->on('en.category_id', '=', 'c.id')->where('en.locale', 'en');
            })
            ->leftJoin('wiki_category_translations as pl', function (JoinClause $join): void {
                $join->on('pl.category_id', '=', 'c.id')->where('pl.locale', 'pl');
            })
            ->leftJoin('wiki_category_translations as parent_en', function (JoinClause $join): void {
                $join->on('parent_en.category_id', '=', 'c.parent_id')->where('parent_en.locale', 'en');
            })
            ->select([
                'c.id',
                'c.parent_id',
                'c.key',
                'c.visible',
                'c.sort_order',
                'c.lock_version',
                'en.name as en_name',
                'pl.name as pl_name',
                'parent_en.name as parent_name',
            ])
            ->selectSub(
                DB::table('wiki_article_category as pivot_count')
                    ->whereColumn('pivot_count.category_id', 'c.id')
                    ->selectRaw('COUNT(*)'),
                'article_count',
            )
            ->orderBy('c.sort_order')
            ->orderBy('c.key')
            ->orderBy('c.id')
            ->get();
    }
}
