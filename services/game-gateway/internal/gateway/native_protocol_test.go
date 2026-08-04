package gateway

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"sort"
	"strings"
	"testing"
)

func TestValidateLoginRequestAcceptsCanonicalOffer(t *testing.T) {
	request := validNativeLoginRequest()
	if err := ValidateLoginRequest(request); err != nil {
		t.Fatalf("canonical offer rejected: %v", err)
	}
}

func TestValidateLoginRequestRejectsMalformedOffers(t *testing.T) {
	tests := []struct {
		name   string
		mutate func(*LoginRequest)
	}{
		{name: "wrong offer version", mutate: func(r *LoginRequest) { r.GameplayOffer.OfferVersion = 2 }},
		{name: "empty build", mutate: func(r *LoginRequest) { r.GameplayOffer.ClientBuild = "" }},
		{name: "non canonical platform", mutate: func(r *LoginRequest) { r.GameplayOffer.ClientPlatform = "Windows" }},
		{name: "zero candidates", mutate: func(r *LoginRequest) { r.GameplayOffer.Candidates = nil }},
		{name: "nine candidates", mutate: func(r *LoginRequest) {
			candidate := r.GameplayOffer.Candidates[0]
			r.GameplayOffer.Candidates = make([]GameplayOfferCandidate, 9)
			for index := range r.GameplayOffer.Candidates {
				r.GameplayOffer.Candidates[index] = candidate
				r.GameplayOffer.Candidates[index].Profile = "profile." + string(rune('a'+index))
			}
		}},
		{name: "duplicate tuple", mutate: func(r *LoginRequest) {
			r.GameplayOffer.Candidates = append(r.GameplayOffer.Candidates, r.GameplayOffer.Candidates[0])
		}},
		{name: "uppercase hash", mutate: func(r *LoginRequest) { r.GameplayOffer.Candidates[0].SchemaSHA256 = strings.Repeat("A", 64) }},
		{name: "unsorted capabilities", mutate: func(r *LoginRequest) { r.GameplayOffer.Candidates[0].Capabilities = []string{"z.v1", "a.v1"} }},
		{name: "duplicate capabilities", mutate: func(r *LoginRequest) { r.GameplayOffer.Candidates[0].Capabilities = []string{"a.v1", "a.v1"} }},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			request := validNativeLoginRequest()
			test.mutate(&request)
			if err := ValidateLoginRequest(request); !errors.Is(err, ErrInvalidRequest) {
				t.Fatalf("expected invalid request, got %v", err)
			}
		})
	}
}

func TestSelectGameplayCandidateIntersectsOptionalCapabilitiesAndUsesStableDigest(t *testing.T) {
	offer := validNativeLoginRequest().GameplayOffer
	offer.Candidates[0].Capabilities = append(offer.Candidates[0].Capabilities, "state.optional-a.v1", "state.optional-c.v1")
	sort.Strings(offer.Candidates[0].Capabilities)
	policy := GameplayPolicy{
		Revision:  41,
		ChannelID: 1,
		Candidates: []GameplayPolicyCandidate{{
			Family: "oteryn", Profile: "oteryn.native.v1", Transport: "tcp.tls13.protobuf.be32.v1",
			SchemaRevision: 1, SchemaSHA256: canonicalNativeSchemaSHA256,
			RequiredCapabilities: nativeV1BaseCapabilities,
			OptionalCapabilities: []string{"state.optional-a.v1", "state.optional-b.v1", "state.optional-c.v1"},
			EndpointID:           "native-1", Host: "game.example.test", Port: 7173, TLSServerName: "game.example.test",
		}},
	}

	selection, err := SelectGameplayCandidate(policy, *offer)
	if err != nil {
		t.Fatalf("selection failed: %v", err)
	}
	if !containsSorted(selection.Capabilities, "state.optional-a.v1") || containsSorted(selection.Capabilities, "state.optional-b.v1") || !containsSorted(selection.Capabilities, "state.optional-c.v1") {
		t.Fatalf("unexpected capability intersection: %#v", selection.Capabilities)
	}

	hash := sha256.New()
	for _, capability := range selection.Capabilities {
		_, _ = hash.Write([]byte(capability))
		_, _ = hash.Write([]byte{'\n'})
	}
	if selection.CapabilityDigestSHA256 != hex.EncodeToString(hash.Sum(nil)) {
		t.Fatalf("unexpected capability digest: %s", selection.CapabilityDigestSHA256)
	}
}

func TestSelectGameplayCandidateRejectsNativeProfileMissingBaseCapability(t *testing.T) {
	offer := validNativeLoginRequest().GameplayOffer
	policy := GameplayPolicy{Revision: 1, ChannelID: 1, Candidates: []GameplayPolicyCandidate{{
		Family: "oteryn", Profile: "oteryn.native.v1", Transport: "tcp.tls13.protobuf.be32.v1",
		SchemaRevision: 1, SchemaSHA256: canonicalNativeSchemaSHA256,
		RequiredCapabilities: nativeV1BaseCapabilities[:len(nativeV1BaseCapabilities)-1],
		EndpointID:           "native-1", Host: "game.example.test", Port: 7173, TLSServerName: "game.example.test",
	}}}

	_, err := SelectGameplayCandidate(policy, *offer)
	if !errors.Is(err, ErrUnavailable) {
		t.Fatalf("expected invalid authoritative policy to fail closed, got %v", err)
	}
}

func TestSelectGameplayCandidateRejectsWrongNativeSchemaIdentity(t *testing.T) {
	offer := validNativeLoginRequest().GameplayOffer
	policy := GameplayPolicy{Revision: 1, ChannelID: 1, Candidates: []GameplayPolicyCandidate{{
		Family: "oteryn", Profile: "oteryn.native.v1", Transport: "tcp.tls13.protobuf.be32.v1",
		SchemaRevision: 1, SchemaSHA256: strings.Repeat("a", 64),
		RequiredCapabilities: nativeV1BaseCapabilities,
		EndpointID:           "native-1", Host: "game.example.test", Port: 7173, TLSServerName: "game.example.test",
	}}}

	_, err := SelectGameplayCandidate(policy, *offer)
	if !errors.Is(err, ErrUnavailable) {
		t.Fatalf("expected noncanonical native schema to fail closed, got %v", err)
	}
}

func TestNewV2SessionRequestBindsExactAuthority(t *testing.T) {
	selection := GameplaySelection{
		PolicyRevision: 3,
		Family:         "oteryn", Profile: "oteryn.native.v1", Transport: "tcp.tls13.protobuf.be32.v1",
		SchemaRevision: 1, SchemaSHA256: canonicalNativeSchemaSHA256,
		Capabilities: nativeV1BaseCapabilities, CapabilityDigestSHA256: capabilityDigest(nativeV1BaseCapabilities),
		EndpointID: "native-1", Host: "game.example.test", Port: 7173, TLSServerName: "game.example.test",
	}
	request, err := NewV2SessionRequest(
		Authorization{CanaryAccountID: 1001, SecurityGeneration: 9},
		World{ID: 7, Host: "legacy.example.test", Port: 7172},
		strings.Repeat("a", 32),
		selection,
	)
	if err != nil {
		t.Fatalf("NewV2SessionRequest failed: %v", err)
	}
	if request.ContractVersion != 2 || request.CanaryAccountID != 1001 || request.SecurityGeneration != 9 || request.WorldID != 7 || request.ChannelID != 1 || request.EndpointID != "native-1" {
		t.Fatalf("missing authority binding: %#v", request)
	}
	if request.Audience != "otheryn-world:7:channel:1:endpoint:native-1" || !request.SingleAdmission || request.CharacterBindingMode != "bind_on_first_admission" {
		t.Fatalf("unexpected admission contract: %#v", request)
	}
}

func validNativeLoginRequest() LoginRequest {
	capabilities := append([]string(nil), nativeV1BaseCapabilities...)
	return LoginRequest{
		ProtocolVersion: 1,
		GameLoginTicket: "ticket",
		GameplayOffer: &GameplayOffer{
			OfferVersion:   1,
			ClientBuild:    "oteryn-client-test",
			ClientPlatform: "windows-x86_64",
			Candidates: []GameplayOfferCandidate{{
				Family: "oteryn", Profile: "oteryn.native.v1", Transport: "tcp.tls13.protobuf.be32.v1",
				SchemaRevision: 1, SchemaSHA256: canonicalNativeSchemaSHA256, Capabilities: capabilities,
			}},
		},
	}
}
