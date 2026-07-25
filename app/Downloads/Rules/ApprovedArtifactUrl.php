<?php

namespace App\Downloads\Rules;

use App\Downloads\Security\ArtifactUrlPolicy;
use Closure;
use Illuminate\Contracts\Validation\ValidationRule;

final readonly class ApprovedArtifactUrl implements ValidationRule
{
    public function __construct(private ArtifactUrlPolicy $policy = new ArtifactUrlPolicy) {}

    public function validate(string $attribute, mixed $value, Closure $fail): void
    {
        if (! is_string($value)) {
            $fail("The {$attribute} must be a valid approved artifact URL.");

            return;
        }

        $reason = $this->policy->rejectionReason($value);

        if ($reason !== null) {
            $fail("The {$attribute} {$reason}");
        }
    }
}
