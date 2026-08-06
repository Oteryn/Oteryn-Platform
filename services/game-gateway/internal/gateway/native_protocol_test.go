package gateway

import (
	"bytes"
	"encoding/json"
	"errors"
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

func TestValidateLoginRequestRejectsForbiddenIdentityKeysEvenWhenNull(t *testing.T) {
	tests := []struct {
		name   string
		value  LoginRequest
		family string
		field  string
	}{
		{name: "native profile null", value: validNativeLoginRequest(), family: "oteryn", field: "profile"},
		{name: "canary native version null", value: validCanaryLoginRequest(), family: "canary", field: "native_protocol_version"},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			payload := marshalWithNullIdentityField(t, test.value, test.family, test.field)
			var request LoginRequest
			if err := json.Unmarshal(payload, &request); err != nil {
				t.Fatalf("decode request: %v", err)
			}
			if err := ValidateLoginRequest(request); !errors.Is(err, ErrInvalidRequest) {
				t.Fatalf("expected forbidden identity key to fail closed, got %v", err)
			}
		})
	}
}

func TestSelectGameplayCandidateRejectsForbiddenPolicyIdentityKeysEvenWhenNull(t *testing.T) {
	tests := []struct {
		name   string
		policy GameplayPolicy
		offer  GameplayOffer
		family string
		field  string
	}{
		{name: "native profile null", policy: validNativePolicy(), offer: *validNativeLoginRequest().GameplayOffer, family: "oteryn", field: "profile"},
		{name: "canary native version null", policy: validCanaryPolicy(), offer: *validCanaryLoginRequest().GameplayOffer, family: "canary", field: "native_protocol_version"},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			payload := marshalWithNullIdentityField(t, test.policy, test.family, test.field)
			var policy GameplayPolicy
			if err := json.Unmarshal(payload, &policy); err != nil {
				t.Fatalf("decode policy: %v", err)
			}
			if _, err := SelectGameplayCandidate(policy, test.offer); !errors.Is(err, ErrUnavailable) {
				t.Fatalf("expected forbidden policy identity key to fail closed, got %v", err)
			}
		})
	}
}

func TestSelectGameplayCandidateUsesExactNativeCapabilitiesAndStableDigest(t *testing.T) {
	selection, err := SelectGameplayCandidate(validNativePolicy(), *validNativeLoginRequest().GameplayOffer)
	if err != nil {
		t.Fatalf("selection failed: %v", err)
	}
	if !equalStringSlices(selection.Capabilities, nativeV1BaseCapabilities) {
		t.Fatalf("unexpected selected capabilities: %#v", selection.Capabilities)
	}
	if selection.CapabilityDigestSHA256 != capabilityDigest(nativeV1BaseCapabilities) {
		t.Fatalf("unexpected capability digest: %s", selection.CapabilityDigestSHA256)
	}
}

func TestValidateLoginRequestRejectsNoncanonicalNativeCapabilities(t *testing.T) {
	tests := []struct {
		name   string
		mutate func([]string) []string
	}{
		{name: "missing capability", mutate: func(values []string) []string { return values[:len(values)-1] }},
		{name: "additional capability", mutate: func(values []string) []string { return append(values, "zz.additional.v1") }},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			request := validNativeLoginRequest()
			capabilities := append([]string(nil), request.GameplayOffer.Candidates[0].Capabilities...)
			request.GameplayOffer.Candidates[0].Capabilities = test.mutate(capabilities)
			if err := ValidateLoginRequest(request); !errors.Is(err, ErrInvalidRequest) {
				t.Fatalf("expected noncanonical native offer to fail closed, got %v", err)
			}
		})
	}
}

func TestSelectGameplayCandidateRejectsNoncanonicalNativePolicyCapabilities(t *testing.T) {
	tests := []struct {
		name   string
		mutate func(*GameplayPolicyCandidate)
	}{
		{name: "missing required capability", mutate: func(candidate *GameplayPolicyCandidate) {
			candidate.RequiredCapabilities = candidate.RequiredCapabilities[:len(candidate.RequiredCapabilities)-1]
		}},
		{name: "additional required capability", mutate: func(candidate *GameplayPolicyCandidate) {
			candidate.RequiredCapabilities = append(candidate.RequiredCapabilities, "zz.additional.v1")
		}},
		{name: "optional capability", mutate: func(candidate *GameplayPolicyCandidate) {
			candidate.OptionalCapabilities = []string{"zz.optional.v1"}
		}},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			policy := validNativePolicy()
			test.mutate(&policy.Candidates[0])
			if _, err := SelectGameplayCandidate(policy, *validNativeLoginRequest().GameplayOffer); !errors.Is(err, ErrUnavailable) {
				t.Fatalf("expected noncanonical authoritative policy to fail closed, got %v", err)
			}
		})
	}
}

func TestSelectGameplayCandidateRejectsWrongNativeSchemaIdentity(t *testing.T) {
	policy := validNativePolicy()
	policy.Candidates[0].SchemaSHA256 = strings.Repeat("a", 64)

	_, err := SelectGameplayCandidate(policy, *validNativeLoginRequest().GameplayOffer)
	if !errors.Is(err, ErrUnavailable) {
		t.Fatalf("expected noncanonical native schema to fail closed, got %v", err)
	}
}

func TestNewV2SessionRequestBindsExactAuthority(t *testing.T) {
	selection := GameplaySelection{
		PolicyRevision: 3,
		Family:         "oteryn", NativeProtocolVersion: 1, Transport: "tcp.tls13.protobuf.be32.v1",
		SchemaRevision: 2, SchemaSHA256: canonicalNativeSchemaSHA256,
		Capabilities: nativeV1BaseCapabilities, CapabilityDigestSHA256: capabilityDigest(nativeV1BaseCapabilities),
		EndpointID: "native-1", Host: "game.example.test", Port: 7173, TLSServerName: "game.example.test",
	}
	request, err := NewV2SessionRequest(
		Authorization{CanaryAccountID: 1001, SecurityGeneration: 0},
		World{ID: 7, Host: "legacy.example.test", Port: 7172},
		strings.Repeat("a", 32),
		selection,
	)
	if err != nil {
		t.Fatalf("NewV2SessionRequest failed: %v", err)
	}
	if request.ContractVersion != 2 || request.CanaryAccountID != 1001 || request.SecurityGeneration != 0 || request.WorldID != 7 || request.ChannelID != 1 || request.EndpointID != "native-1" {
		t.Fatalf("missing authority binding: %#v", request)
	}
	if request.Audience != "otheryn-world:7:channel:1:endpoint:native-1" || !request.SingleAdmission || request.CharacterBindingMode != "bind_on_first_admission" {
		t.Fatalf("unexpected admission contract: %#v", request)
	}
}

func TestNewV2SessionRequestRejectsNegativeSecurityGeneration(t *testing.T) {
	selection := GameplaySelection{
		PolicyRevision: 3,
		Family:         "oteryn", NativeProtocolVersion: 1, Transport: "tcp.tls13.protobuf.be32.v1",
		SchemaRevision: 2, SchemaSHA256: canonicalNativeSchemaSHA256,
		Capabilities: nativeV1BaseCapabilities, CapabilityDigestSHA256: capabilityDigest(nativeV1BaseCapabilities),
		EndpointID: "native-1", Host: "game.example.test", Port: 7173, TLSServerName: "game.example.test",
	}
	_, err := NewV2SessionRequest(
		Authorization{CanaryAccountID: 1001, SecurityGeneration: -1},
		World{ID: 7, Host: "legacy.example.test", Port: 7172},
		strings.Repeat("a", 32),
		selection,
	)
	if !errors.Is(err, ErrUnavailable) {
		t.Fatalf("expected negative generation to fail closed, got %v", err)
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
				Family: "oteryn", NativeProtocolVersion: 1, Transport: "tcp.tls13.protobuf.be32.v1",
				SchemaRevision: 2, SchemaSHA256: canonicalNativeSchemaSHA256, Capabilities: capabilities,
			}},
		},
	}
}

func validNativePolicy() GameplayPolicy {
	return GameplayPolicy{
		Revision:  41,
		ChannelID: 1,
		Candidates: []GameplayPolicyCandidate{{
			Family: "oteryn", NativeProtocolVersion: 1, Transport: "tcp.tls13.protobuf.be32.v1",
			SchemaRevision: 2, SchemaSHA256: canonicalNativeSchemaSHA256,
			RequiredCapabilities: append([]string(nil), nativeV1BaseCapabilities...),
			OptionalCapabilities: []string{},
			EndpointID:           "native-1", Host: "game.example.test", Port: 7173, TLSServerName: "game.example.test",
		}},
	}
}

func validCanaryLoginRequest() LoginRequest {
	return LoginRequest{
		ProtocolVersion: 1,
		GameLoginTicket: "ticket",
		GameplayOffer: &GameplayOffer{
			OfferVersion:   1,
			ClientBuild:    "oteryn-client-test",
			ClientPlatform: "windows-x86_64",
			Candidates: []GameplayOfferCandidate{{
				Family: "canary", Profile: "canary.current", Transport: "canary.sequence.v1",
				SchemaRevision: 1, SchemaSHA256: strings.Repeat("a", 64), Capabilities: []string{"session.single-admission.v1"},
			}},
		},
	}
}

func validCanaryPolicy() GameplayPolicy {
	return GameplayPolicy{
		Revision:  41,
		ChannelID: 1,
		Candidates: []GameplayPolicyCandidate{{
			Family: "canary", Profile: "canary.current", Transport: "canary.sequence.v1",
			SchemaRevision: 1, SchemaSHA256: strings.Repeat("a", 64),
			RequiredCapabilities: []string{"session.single-admission.v1"}, OptionalCapabilities: []string{},
			EndpointID: "canary-1", Host: "game.example.test", Port: 7172, TLSServerName: "game.example.test",
		}},
	}
}

func marshalWithNullIdentityField(t *testing.T, value any, family, field string) []byte {
	t.Helper()

	payload, err := json.Marshal(value)
	if err != nil {
		t.Fatalf("encode value: %v", err)
	}
	anchor := []byte("\"family\":\"" + family + "\"")
	replacement := []byte("\"family\":\"" + family + "\",\"" + field + "\":null")
	if !bytes.Contains(payload, anchor) {
		t.Fatalf("missing family anchor %q in %s", anchor, payload)
	}
	return bytes.Replace(payload, anchor, replacement, 1)
}
