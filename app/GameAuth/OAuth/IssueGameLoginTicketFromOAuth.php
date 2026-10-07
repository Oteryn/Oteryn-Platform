<?php

namespace App\GameAuth\OAuth;

use App\GameAuth\Tickets\IssuedGameLoginTicket;
use App\GameAuth\Tickets\IssueGameLoginTicket;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Laravel\Passport\RefreshToken;

final class IssueGameLoginTicketFromOAuth
{
    public function __construct(
        private readonly IssueGameLoginTicket $tickets,
        private readonly VerifyNativeOAuthAccess $access,
    ) {}

    public function execute(Identity $identity, string $accessTokenId): IssuedGameLoginTicket
    {
        return DB::transaction(function () use ($identity, $accessTokenId): IssuedGameLoginTicket {
            $verified = $this->access->locked($identity, $accessTokenId);
            $issued = $this->tickets->execute($verified->identity);

            RefreshToken::query()
                ->where('access_token_id', $verified->token->getKey())
                ->where('revoked', false)
                ->update(['revoked' => true]);

            $verified->token->forceFill(['revoked' => true])->save();

            return $issued;
        });
    }
}
