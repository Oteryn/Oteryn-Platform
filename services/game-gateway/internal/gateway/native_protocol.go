package gateway

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"net"
	"sort"
	"strings"
)

const (
	gatewayProtocolVersion       = 1
	gameplayOfferVersion         = 1
	gameSessionContractVersionV2 = 2
	initialGameplayChannelID     = 1
	characterBindingModeFirst    = "bind_on_first_admission"
)

var nativeV1BaseCapabilities = []string{
	"actions.command-result.v1",
	"chat.semantic.v1",
	"combat.server-authoritative.v1",
	"inventory.server-authoritative.v1",
	"ordering.server-sequence.v1",
	"reconciliation.movement.v1",
	"session.single-admission.v1",
	"state.revision.v1",
	"state.snapshot-delta.v1",
}

func ValidateLoginRequest(request LoginRequest) error {
	if request.ProtocolVersion != gatewayProtocolVersion || request.GameLoginTicket == "" || len(request.GameLoginTicket) > 1024 {
		return ErrInvalidRequest
	}
	if request.GameplayOffer == nil {
		return nil
	}

	offer := request.GameplayOffer
	if offer.OfferVersion != gameplayOfferVersion || !isPrintableASCII(offer.ClientBuild, 1, 64) || !isIdentifier(offer.ClientPlatform) {
		return ErrInvalidRequest
	}
	if len(offer.Candidates) < 1 || len(offer.Candidates) > 8 {
		return ErrInvalidRequest
	}

	seen := make(map[string]struct{}, len(offer.Candidates))
	for _, candidate := range offer.Candidates {
		if !validOfferedCandidate(candidate) {
			return ErrInvalidRequest
		}
		key := candidateTupleKey(candidate.Family, candidate.Profile, candidate.Transport, candidate.SchemaRevision, candidate.SchemaSHA256)
		if _, exists := seen[key]; exists {
			return ErrInvalidRequest
		}
		seen[key] = struct{}{}
	}

	return nil
}

func SelectGameplayCandidate(policy GameplayPolicy, offer GameplayOffer) (GameplaySelection, error) {
	if policy.Revision < 1 || policy.ChannelID != initialGameplayChannelID || len(policy.Candidates) > 32 {
		return GameplaySelection{}, ErrUnavailable
	}

	offered := make(map[string]GameplayOfferCandidate, len(offer.Candidates))
	for _, candidate := range offer.Candidates {
		offered[candidateTupleKey(candidate.Family, candidate.Profile, candidate.Transport, candidate.SchemaRevision, candidate.SchemaSHA256)] = candidate
	}

	for _, candidate := range policy.Candidates {
		if !validPolicyCandidate(candidate) {
			return GameplaySelection{}, ErrUnavailable
		}

		clientCandidate, exists := offered[candidateTupleKey(candidate.Family, candidate.Profile, candidate.Transport, candidate.SchemaRevision, candidate.SchemaSHA256)]
		if !exists || !containsEvery(clientCandidate.Capabilities, candidate.RequiredCapabilities) {
			continue
		}

		selectedCapabilities := append([]string(nil), candidate.RequiredCapabilities...)
		for _, capability := range candidate.OptionalCapabilities {
			if containsSorted(clientCandidate.Capabilities, capability) {
				selectedCapabilities = append(selectedCapabilities, capability)
			}
		}
		sort.Strings(selectedCapabilities)

		return GameplaySelection{
			PolicyRevision:         policy.Revision,
			Family:                 candidate.Family,
			Profile:                candidate.Profile,
			Transport:              candidate.Transport,
			SchemaRevision:         candidate.SchemaRevision,
			SchemaSHA256:           candidate.SchemaSHA256,
			Capabilities:           selectedCapabilities,
			CapabilityDigestSHA256: capabilityDigest(selectedCapabilities),
			EndpointID:             candidate.EndpointID,
			Host:                   candidate.Host,
			Port:                   candidate.Port,
			TLSServerName:          candidate.TLSServerName,
		}, nil
	}

	return GameplaySelection{}, ErrUnsupportedGameplayPair
}

func NewV2SessionRequest(authorization Authorization, world World, loginAttemptID string, selection GameplaySelection) (SessionRequest, error) {
	if authorization.CanaryAccountID < 1 || authorization.SecurityGeneration < 1 || world.ID < 1 || len(loginAttemptID) != 32 {
		return SessionRequest{}, ErrUnavailable
	}
	if _, err := hex.DecodeString(loginAttemptID); err != nil {
		return SessionRequest{}, ErrUnavailable
	}

	return SessionRequest{
		ContractVersion:      gameSessionContractVersionV2,
		CanaryAccountID:      authorization.CanaryAccountID,
		SecurityGeneration:   authorization.SecurityGeneration,
		WorldID:              world.ID,
		ChannelID:            initialGameplayChannelID,
		LoginAttemptID:       loginAttemptID,
		WorldPolicyRevision:  selection.PolicyRevision,
		EndpointID:           selection.EndpointID,
		Audience:             fmt.Sprintf("otheryn-world:%d:channel:%d:endpoint:%s", world.ID, initialGameplayChannelID, selection.EndpointID),
		CharacterBindingMode: characterBindingModeFirst,
		SingleAdmission:      true,
		GameplaySelection:    &selection,
	}, nil
}

func validOfferedCandidate(candidate GameplayOfferCandidate) bool {
	return isIdentifier(candidate.Family)
		&& isIdentifier(candidate.Profile)
		&& isIdentifier(candidate.Transport)
		&& candidate.SchemaRevision > 0
		&& isLowerHexSHA256(candidate.SchemaSHA256)
		&& isCanonicalCapabilities(candidate.Capabilities)
}

func validPolicyCandidate(candidate GameplayPolicyCandidate) bool {
	if !isIdentifier(candidate.Family)
		|| !isIdentifier(candidate.Profile)
		|| !isIdentifier(candidate.Transport)
		|| candidate.SchemaRevision == 0
		|| !isLowerHexSHA256(candidate.SchemaSHA256)
		|| !isCanonicalCapabilities(candidate.RequiredCapabilities)
		|| !isCanonicalCapabilities(candidate.OptionalCapabilities)
		|| !isIdentifier(candidate.EndpointID)
		|| !isHost(candidate.Host)
		|| candidate.Port < 1
		|| candidate.Port > 65535
		|| !isHost(candidate.TLSServerName)
		|| strings.Contains(candidate.TLSServerName, "*") {
		return false
	}
	if intersects(candidate.RequiredCapabilities, candidate.OptionalCapabilities) {
		return false
	}

	if candidate.Family == "oteryn" && candidate.Profile == "oteryn.native.v1" {
		if candidate.Transport != "tcp.tls13.protobuf.be32.v1" || candidate.SchemaRevision != 1 {
			return false
		}
		if !containsEvery(candidate.RequiredCapabilities, nativeV1BaseCapabilities) {
			return false
		}
	}

	return true
}

func capabilityDigest(capabilities []string) string {
	hash := sha256.New()
	for _, capability := range capabilities {
		_, _ = hash.Write([]byte(capability))
		_, _ = hash.Write([]byte{'\n'})
	}
	return hex.EncodeToString(hash.Sum(nil))
}

func candidateTupleKey(family, profile, transport string, schemaRevision uint32, schemaSHA256 string) string {
	return fmt.Sprintf("%s\x00%s\x00%s\x00%d\x00%s", family, profile, transport, schemaRevision, schemaSHA256)
}

func isIdentifier(value string) bool {
	if len(value) < 1 || len(value) > 64 || !isLowerAlphaNumeric(value[0]) {
		return false
	}
	for index := 1; index < len(value); index++ {
		character := value[index]
		if !isLowerAlphaNumeric(character) && character != '.' && character != '_' && character != '-' {
			return false
		}
	}
	return true
}

func isLowerAlphaNumeric(character byte) bool {
	return character >= 'a' && character <= 'z' || character >= '0' && character <= '9'
}

func isPrintableASCII(value string, minimum, maximum int) bool {
	if len(value) < minimum || len(value) > maximum {
		return false
	}
	for index := 0; index < len(value); index++ {
		if value[index] < 0x20 || value[index] > 0x7e {
			return false
		}
	}
	return true
}

func isLowerHexSHA256(value string) bool {
	if len(value) != 64 {
		return false
	}
	for index := 0; index < len(value); index++ {
		character := value[index]
		if !(character >= '0' && character <= '9') && !(character >= 'a' && character <= 'f') {
			return false
		}
	}
	return true
}

func isCanonicalCapabilities(capabilities []string) bool {
	if len(capabilities) > 64 {
		return false
	}
	for index, capability := range capabilities {
		if !isIdentifier(capability) {
			return false
		}
		if index > 0 && capabilities[index-1] >= capability {
			return false
		}
	}
	return true
}

func containsEvery(haystack, needles []string) bool {
	for _, needle := range needles {
		if !containsSorted(haystack, needle) {
			return false
		}
	}
	return true
}

func containsSorted(values []string, target string) bool {
	index := sort.SearchStrings(values, target)
	return index < len(values) && values[index] == target
}

func intersects(left, right []string) bool {
	for _, value := range left {
		if containsSorted(right, value) {
			return true
		}
	}
	return false
}

func isHost(value string) bool {
	if len(value) < 1 || len(value) > 253 || strings.ContainsAny(value, "/:@?#") {
		return false
	}
	if net.ParseIP(value) != nil {
		return true
	}
	labels := strings.Split(value, ".")
	for _, label := range labels {
		if len(label) < 1 || len(label) > 63 || label[0] == '-' || label[len(label)-1] == '-' {
			return false
		}
		for index := 0; index < len(label); index++ {
			character := label[index]
			if !(character >= 'A' && character <= 'Z') && !(character >= 'a' && character <= 'z') && !(character >= '0' && character <= '9') && character != '-' {
				return false
			}
		}
	}
	return true
}
