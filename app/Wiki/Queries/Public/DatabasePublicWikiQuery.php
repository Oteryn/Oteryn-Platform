<?php

namespace App\Wiki\Queries\Public;

use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Domain\WikiContentRules;
use App\Wiki\ViewModels\Public\WikiArticleCard;
use App\Wiki\ViewModels\Public\WikiArticlePageViewModel;
use App\Wiki\ViewModels\Public\WikiBreadcrumb;
use App\Wiki\ViewModels\Public\WikiCategoryCard;
use App\Wiki\ViewModels\Public\WikiCategoryPageViewModel;
use App\Wiki\ViewModels\Public\WikiHomeViewModel;
use Illuminate\Database\Query\Builder;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use stdClass;
use UnexpectedValueException;

final class DatabasePublicWikiQuery implements PublicWikiQuery
{
    public function home(string $locale): WikiHomeViewModel
    {
        WikiContentRules::assertSupportedLocale($locale);

        return new WikiHomeViewModel(
            $this->categoryCards($locale, null),
            $this->articleCards($locale, null, true, 4),
            $this->articleCards($locale, null, false, 8),
        );
    }

    public function category(string $locale, string $slug): ?WikiCategoryPageViewModel
    {
        WikiContentRules::assertSupportedLocale($locale);

        $category = $this->visibleCategories($locale)
            ->where('ct.slug', $slug)
            ->select(['c.id', 'c.parent_id', 'ct.name', 'ct.slug', 'ct.description'])
            ->first();
        if (! $category instanceof stdClass) {
            return null;
        }

        $categoryId = $this->integer($category->id);

        return new WikiCategoryPageViewModel(
            $categoryId,
            $this->string($category->name),
            $this->string($category->slug),
            $this->nullableString($category->description),
            $this->categoryCards($locale, $categoryId),
            $this->articleCards($locale, $categoryId, false, 100),
            $this->categoryBreadcrumbs($locale, $categoryId, $this->string($category->name)),
        );
    }

    public function article(string $locale, string $slug): ?WikiArticlePageViewModel
    {
        WikiContentRules::assertSupportedLocale($locale);

        $article = $this->publishedArticles($locale)
            ->where('wt.slug', $slug)
            ->select([
                'a.id',
                'wt.title',
                'wt.slug',
                'wt.summary',
                'wt.source_markdown',
                'a.published_at',
            ])
            ->first();
        if (! $article instanceof stdClass) {
            return null;
        }

        $articleId = $this->integer($article->id);
        $categories = $this->articleCategories($locale, $articleId);
        $primaryCategory = $categories[0] ?? null;
        $breadcrumbs = $primaryCategory instanceof WikiCategoryCard
            ? $this->categoryBreadcrumbs($locale, $primaryCategory->id, $primaryCategory->name)
            : [
                new WikiBreadcrumb((string) __('public.navigation.home'), route('localized.home', ['locale' => $locale])),
                new WikiBreadcrumb((string) __('public.wiki.title'), route('wiki.index', ['locale' => $locale])),
            ];
        $breadcrumbs[] = new WikiBreadcrumb($this->string($article->title), null);

        return new WikiArticlePageViewModel(
            $articleId,
            $this->string($article->title),
            $this->string($article->slug),
            $this->string($article->summary),
            $this->string($article->source_markdown),
            Carbon::parse($this->string($article->published_at)),
            $categories,
            $this->relatedArticles($locale, $articleId),
            $breadcrumbs,
        );
    }

    public function equivalentArticleSlug(int $articleId, string $locale): ?string
    {
        WikiContentRules::assertSupportedLocale($locale);

        $slug = $this->publishedArticles($locale)
            ->where('a.id', $articleId)
            ->value('wt.slug');

        return is_string($slug) ? $slug : null;
    }

    public function equivalentCategorySlug(int $categoryId, string $locale): ?string
    {
        WikiContentRules::assertSupportedLocale($locale);

        $slug = $this->visibleCategories($locale)
            ->where('c.id', $categoryId)
            ->value('ct.slug');

        return is_string($slug) ? $slug : null;
    }

    public function publishedArticleId(string $locale, string $slug): ?int
    {
        WikiContentRules::assertSupportedLocale($locale);

        $id = $this->publishedArticles($locale)
            ->where('wt.slug', $slug)
            ->value('a.id');

        return $this->nullableInteger($id);
    }

    public function visibleCategoryId(string $locale, string $slug): ?int
    {
        WikiContentRules::assertSupportedLocale($locale);

        $id = $this->visibleCategories($locale)
            ->where('ct.slug', $slug)
            ->value('c.id');

        return $this->nullableInteger($id);
    }

    /** @return list<WikiCategoryCard> */
    private function categoryCards(string $locale, ?int $parentId): array
    {
        $query = $this->visibleCategories($locale)
            ->where('c.parent_id', $parentId)
            ->select(['c.id', 'ct.name', 'ct.slug', 'ct.description'])
            ->selectSub(
                function (Builder $articles) use ($locale): void {
                    $articles->from('wiki_article_category as count_pivot')
                        ->join('wiki_articles as count_articles', 'count_articles.id', '=', 'count_pivot.article_id')
                        ->join('wiki_article_translations as count_translations', function (JoinClause $join) use ($locale): void {
                            $join->on('count_translations.article_id', '=', 'count_articles.id')
                                ->where('count_translations.locale', $locale);
                        })
                        ->whereColumn('count_pivot.category_id', 'c.id')
                        ->where('count_articles.status', WikiArticleStatus::PUBLISHED->value)
                        ->whereNotNull('count_articles.published_at')
                        ->where('count_articles.published_at', '<=', now())
                        ->selectRaw('COUNT(*)');

                    if ($locale !== 'en') {
                        $articles->join('wiki_article_translations as count_source_translations', function (JoinClause $join): void {
                            $join->on('count_source_translations.article_id', '=', 'count_articles.id')
                                ->where('count_source_translations.locale', 'en');
                        })->whereColumn('count_translations.updated_at', '>=', 'count_source_translations.updated_at');
                    }
                },
                'article_count',
            )
            ->orderBy('c.sort_order')
            ->orderBy('ct.name')
            ->orderBy('c.id');

        return array_values($query->get()
            ->map(fn (stdClass $row): WikiCategoryCard => $this->categoryCard($row))
            ->values()
            ->all());
    }

    /** @return list<WikiArticleCard> */
    private function articleCards(
        string $locale,
        ?int $categoryId,
        bool $featuredOnly,
        int $limit,
    ): array {
        $query = $this->publishedArticles($locale);

        if ($categoryId !== null) {
            $query->join('wiki_article_category as article_pivot', function (JoinClause $join) use ($categoryId): void {
                $join->on('article_pivot.article_id', '=', 'a.id')
                    ->where('article_pivot.category_id', $categoryId);
            });
        }

        if ($featuredOnly) {
            $query->where('a.is_featured', true)
                ->orderBy('a.sort_order');
        } else {
            $query->orderByDesc('a.published_at');
        }

        return array_values($query
            ->select(['a.id', 'wt.title', 'wt.slug', 'wt.summary', 'a.published_at'])
            ->orderBy('wt.title')
            ->orderBy('a.id')
            ->limit($limit)
            ->get()
            ->map(fn (stdClass $row): WikiArticleCard => $this->articleCard($row))
            ->values()
            ->all());
    }

    /** @return list<WikiCategoryCard> */
    private function articleCategories(string $locale, int $articleId): array
    {
        return array_values($this->visibleCategories($locale)
            ->join('wiki_article_category as category_pivot', function (JoinClause $join) use ($articleId): void {
                $join->on('category_pivot.category_id', '=', 'c.id')
                    ->where('category_pivot.article_id', $articleId);
            })
            ->select(['c.id', 'ct.name', 'ct.slug', 'ct.description'])
            ->selectRaw('0 as article_count')
            ->orderBy('category_pivot.sort_order')
            ->orderBy('c.sort_order')
            ->orderBy('ct.name')
            ->get()
            ->map(fn (stdClass $row): WikiCategoryCard => $this->categoryCard($row))
            ->values()
            ->all());
    }

    /** @return list<WikiArticleCard> */
    private function relatedArticles(string $locale, int $articleId): array
    {
        return array_values($this->publishedArticles($locale)
            ->join('wiki_article_category as related_pivot', 'related_pivot.article_id', '=', 'a.id')
            ->join('wiki_categories as related_categories', function (JoinClause $join): void {
                $join->on('related_categories.id', '=', 'related_pivot.category_id')
                    ->where('related_categories.visible', true);
            })
            ->where('a.id', '!=', $articleId)
            ->whereIn('related_pivot.category_id', function (Builder $query) use ($articleId): void {
                $query->from('wiki_article_category')
                    ->where('article_id', $articleId)
                    ->select('category_id');
            })
            ->select(['a.id', 'wt.title', 'wt.slug', 'wt.summary', 'a.published_at'])
            ->selectRaw('COUNT(DISTINCT related_pivot.category_id) as shared_categories')
            ->groupBy(['a.id', 'wt.title', 'wt.slug', 'wt.summary', 'a.published_at', 'a.sort_order'])
            ->orderByDesc('shared_categories')
            ->orderBy('a.sort_order')
            ->orderBy('wt.title')
            ->orderBy('a.id')
            ->limit(5)
            ->get()
            ->map(fn (stdClass $row): WikiArticleCard => $this->articleCard($row))
            ->values()
            ->all());
    }

    /** @return list<WikiBreadcrumb> */
    private function categoryBreadcrumbs(string $locale, int $categoryId, string $currentName): array
    {
        $trail = [];
        $visited = [];
        $cursor = $categoryId;

        while (! isset($visited[$cursor]) && count($visited) < 20) {
            $visited[$cursor] = true;
            $row = $this->visibleCategories($locale)
                ->where('c.id', $cursor)
                ->select(['c.id', 'c.parent_id', 'ct.name', 'ct.slug'])
                ->first();
            if (! $row instanceof stdClass) {
                break;
            }

            array_unshift($trail, new WikiBreadcrumb(
                $this->string($row->name),
                route('wiki.category', ['locale' => $locale, 'slug' => $this->string($row->slug)]),
            ));

            $parentId = $this->nullableInteger($row->parent_id);
            if ($parentId === null) {
                break;
            }
            $cursor = $parentId;
        }

        $breadcrumbs = [
            new WikiBreadcrumb((string) __('public.navigation.home'), route('localized.home', ['locale' => $locale])),
            new WikiBreadcrumb((string) __('public.wiki.title'), route('wiki.index', ['locale' => $locale])),
        ];

        foreach ($trail as $item) {
            $breadcrumbs[] = $item;
        }

        $last = count($breadcrumbs) - 1;
        if ($breadcrumbs[$last]->label === $currentName) {
            $breadcrumbs[$last] = new WikiBreadcrumb($currentName, null);
        }

        return $breadcrumbs;
    }

    private function publishedArticles(string $locale): Builder
    {
        $query = DB::table('wiki_articles as a')
            ->join('wiki_article_translations as wt', function (JoinClause $join) use ($locale): void {
                $join->on('wt.article_id', '=', 'a.id')
                    ->where('wt.locale', $locale);
            });

        if ($locale !== 'en') {
            $query->join('wiki_article_translations as source_wt', function (JoinClause $join): void {
                $join->on('source_wt.article_id', '=', 'a.id')
                    ->where('source_wt.locale', 'en');
            })->whereColumn('wt.updated_at', '>=', 'source_wt.updated_at');
        }

        return $query
            ->where('a.status', WikiArticleStatus::PUBLISHED->value)
            ->whereNotNull('a.published_at')
            ->where('a.published_at', '<=', now());
    }

    private function visibleCategories(string $locale): Builder
    {
        $query = DB::table('wiki_categories as c')
            ->join('wiki_category_translations as ct', function (JoinClause $join) use ($locale): void {
                $join->on('ct.category_id', '=', 'c.id')
                    ->where('ct.locale', $locale);
            });

        if ($locale !== 'en') {
            $query->join('wiki_category_translations as source_ct', function (JoinClause $join): void {
                $join->on('source_ct.category_id', '=', 'c.id')
                    ->where('source_ct.locale', 'en');
            })->whereColumn('ct.updated_at', '>=', 'source_ct.updated_at');
        }

        return $query
            ->where('c.visible', true);
    }

    private function categoryCard(stdClass $row): WikiCategoryCard
    {
        return new WikiCategoryCard(
            $this->integer($row->id),
            $this->string($row->name),
            $this->string($row->slug),
            $this->nullableString($row->description),
            $this->integer($row->article_count),
        );
    }

    private function articleCard(stdClass $row): WikiArticleCard
    {
        return new WikiArticleCard(
            $this->integer($row->id),
            $this->string($row->title),
            $this->string($row->slug),
            $this->string($row->summary),
            Carbon::parse($this->string($row->published_at)),
        );
    }

    private function string(mixed $value): string
    {
        if (! is_string($value) && ! is_int($value) && ! is_float($value)) {
            throw new UnexpectedValueException('Expected a scalar Wiki query value.');
        }

        return (string) $value;
    }

    private function nullableString(mixed $value): ?string
    {
        return $value === null ? null : $this->string($value);
    }

    private function integer(mixed $value): int
    {
        if (is_int($value)) {
            return $value;
        }
        if (! is_string($value) || preg_match('/\A\d+\z/D', $value) !== 1) {
            throw new UnexpectedValueException('Expected an integer Wiki query value.');
        }

        return (int) $value;
    }

    private function nullableInteger(mixed $value): ?int
    {
        if ($value === null) {
            return null;
        }

        return $this->integer($value);
    }
}
