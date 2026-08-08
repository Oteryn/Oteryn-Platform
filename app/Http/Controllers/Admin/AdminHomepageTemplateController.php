<?php

namespace App\Http\Controllers\Admin;

use App\Identity\Models\Identity;
use App\PublicPortal\HomePageQuery;
use App\PublicPortal\HomepageTemplates\HomepageTemplateConflict;
use App\PublicPortal\HomepageTemplates\HomepageTemplateRegistry;
use App\PublicPortal\HomepageTemplates\HomepageTemplateRollbackUnavailable;
use App\PublicPortal\HomepageTemplates\HomepageTemplateStore;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Validation\Rule;
use Illuminate\View\View;

final class AdminHomepageTemplateController
{
    public function __construct(
        private readonly HomepageTemplateRegistry $registry,
        private readonly HomepageTemplateStore $store,
        private readonly HomePageQuery $homePageQuery,
    ) {}

    public function index(): View
    {
        return view('admin.homepage-templates.index', [
            'templates' => $this->registry->all(),
            'snapshot' => $this->store->snapshot(),
        ]);
    }

    public function preview(string $template): Response
    {
        abort_unless($this->registry->has($template), 404);

        return response()
            ->view($this->registry->view($template), [
                'homePage' => $this->homePageQuery->get(),
            ])
            ->header('X-Robots-Tag', 'noindex, nofollow')
            ->header('Cache-Control', 'no-store, private, max-age=0');
    }

    public function activate(Request $request): RedirectResponse
    {
        /** @var array{template: string, version: int} $validated */
        $validated = $request->validate([
            'template' => ['required', 'string', 'max:64', Rule::in($this->registry->keys())],
            'version' => ['required', 'integer', 'min:0'],
        ]);

        try {
            $this->store->activate(
                $this->actor($request),
                $validated['template'],
                $validated['version'],
            );
        } catch (HomepageTemplateConflict) {
            return redirect()
                ->route('admin.homepage-templates.index')
                ->with('error', __('homepage_templates.admin.conflict'));
        }

        return redirect()
            ->route('admin.homepage-templates.index')
            ->with('status', __('homepage_templates.admin.activated'));
    }

    public function rollback(Request $request): RedirectResponse
    {
        /** @var array{version: int} $validated */
        $validated = $request->validate([
            'version' => ['required', 'integer', 'min:0'],
        ]);

        try {
            $this->store->rollback($this->actor($request), $validated['version']);
        } catch (HomepageTemplateConflict) {
            return redirect()
                ->route('admin.homepage-templates.index')
                ->with('error', __('homepage_templates.admin.conflict'));
        } catch (HomepageTemplateRollbackUnavailable) {
            return redirect()
                ->route('admin.homepage-templates.index')
                ->with('error', __('homepage_templates.admin.rollback_unavailable'));
        }

        return redirect()
            ->route('admin.homepage-templates.index')
            ->with('status', __('homepage_templates.admin.rolled_back'));
    }

    private function actor(Request $request): Identity
    {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        return $actor;
    }
}
