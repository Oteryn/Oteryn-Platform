<?php

namespace App\Marketplace\Http;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalEmail;
use App\Marketplace\Actions\RecoverCharacterAuction;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\Actions\AdjustWalletBalance;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;

final class AdminMarketplaceController
{
    public function __construct(
        private readonly AdjustWalletBalance $adjustWallet,
        private readonly RecoverCharacterAuction $recover,
        private readonly AdminAuditRecorder $audit,
    ) {}

    public function index(Request $request): View
    {
        $email = $request->query('email');
        $identity = is_string($email) && $email !== ''
            ? Identity::query()->where('email', CanonicalEmail::normalize($email))->first()
            : null;
        $wallet = $identity instanceof Identity
            ? WalletAccount::query()->whereKey($identity->id)->first()
            : null;
        $ledger = $identity instanceof Identity
            ? WalletLedgerEntry::query()->where('identity_id', $identity->id)->orderByDesc('id')->limit(50)->get()
            : collect();

        return view('admin.marketplace.index', [
            'searchedEmail' => is_string($email) ? $email : '',
            'identity' => $identity,
            'wallet' => $wallet,
            'ledger' => $ledger,
            'recoveryAuctions' => CharacterAuction::query()
                ->where('status', CharacterAuction::STATUS_RECOVERY_REQUIRED)
                ->orderBy('updated_at')
                ->limit(100)
                ->get(),
        ]);
    }

    public function adjust(Request $request): RedirectResponse
    {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        /** @var array{email: string, amount: int, reason: string, request_id: string} $validated */
        $validated = $request->validate([
            'email' => ['required', 'email:rfc', 'max:255'],
            'amount' => ['required', 'integer', 'min:-1000000000', 'max:1000000000', 'not_in:0'],
            'reason' => ['required', 'string', 'min:10', 'max:500'],
            'request_id' => ['required', 'uuid'],
        ]);

        $target = Identity::query()->where('email', CanonicalEmail::normalize($validated['email']))->first();
        if (! $target instanceof Identity) {
            return back()->withInput()->withErrors(['email' => 'No Platform Identity exists for this email address.']);
        }

        try {
            $this->adjustWallet->execute(
                $actor,
                $target,
                $validated['amount'],
                $validated['reason'],
                $validated['request_id'],
            );
        } catch (MarketplaceException $exception) {
            return back()->withInput()->withErrors(['marketplace' => $exception->getMessage()]);
        }

        return redirect()->route('admin.marketplace.index', ['email' => $target->email])
            ->with('status', 'Wallet adjustment recorded.');
    }

    public function recover(Request $request, CharacterAuction $auction): RedirectResponse
    {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        try {
            $result = $this->recover->execute($auction);
        } catch (MarketplaceException $exception) {
            return back()->withErrors(['marketplace' => $exception->getMessage()]);
        }

        $this->audit->record(
            $actor->id,
            'marketplace.auction_recovery_requested',
            'character_auction',
            (string) $auction->id,
            ['result_status' => $result->status, 'result_saga_state' => $result->saga_state],
        );

        return back()->with('status', 'Marketplace recovery step completed.');
    }
}
