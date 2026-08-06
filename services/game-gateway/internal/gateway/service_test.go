package gateway

import (
	"context"
	"errors"
	"sort"
	"strings"
	"testing"
	"time"
)

type fakePlatform struct {
	authorization Authorization
	loginContext  LoginContext
	redeemErr     error
	contextErr    error
	readyErr      error
	redeemCalls   int
	contextCalls  int
}

func (f *fakePlatform) Redeem(context.Context, string) (Authorization, error) {
	f.redeemCalls++
	return f.authorization, f.redeemErr
}

func (f *fakePlatform) LoginContext(context.Context, int64) (LoginContext, error) {
	f.contextCalls++
	return f.loginContext, f.contextErr
}

func (f *fakePlatform) Ready(context.Context) error { return f.readyErr }

type fakeSessionIssuer struct {
	session       Session
	err           error
	readyErr      error
	readyForErr   error
	calls         int
	readyForCalls int
	request       SessionRequest
	readyRequest  SessionRequest
}

func (f *fakeSessionIssuer) Create(_ context.Context, request SessionRequest) (Session, error) {
	f.calls++
	f.request = request
	return f.session, f.err
}

func (f *fakeSessionIssuer) Ready(context.Context) error { return f.readyErr }

func (f *fakeSessionIssuer) ReadyFor(_ context.Context, request SessionRequest) error {
	f.readyForCalls++
	f.readyRequest = request
	return f.readyForErr
}

func TestLoginSuccessPreservesLegacySessionContract(t *testing.T) {
	now := time.Date(2026, 7, 22, 8, 0, 0, 0, time.UTC)
	platform := successfulPlatform()
	sessions := &fakeSessionIssuer{session: Session{Credential: "session-secret", ExpiresAt: now.Add(time.Minute)}}
	service := NewService(platform, sessions)
	service.now = func() time.Time { return now }

	response, err := service.Login(context.Background(), "one-time-ticket")
	if err != nil {
		t.Fatalf("Login returned error: %v", err)
	}
	if response.ProtocolVersion != 1 || response.GameSessionContractVersion != 0 || response.GameplaySelection != nil {
		t.Fatalf("unexpected legacy response: %#v", response)
	}
	if platform.redeemCalls != 1 || platform.contextCalls != 1 || sessions.calls != 1 || sessions.readyForCalls != 0 {
		t.Fatalf("unexpected dependency calls: redeem=%d context=%d session=%d readyFor=%d", platform.redeemCalls, platform.contextCalls, sessions.calls, sessions.readyForCalls)
	}
	if sessions.request.ContractVersion != 1 || sessions.request.CanaryAccountID != 1001 || sessions.request.WorldID != 1 || sessions.request.LoginAttemptID == "" {
		t.Fatalf("unexpected legacy session request: %#v", sessions.request)
	}
}

func TestLoginWithOfferUsesAuthoritativePolicyOrderAndBindsV2(t *testing.T) {
	now := time.Date(2026, 8, 4, 12, 0, 0, 0, time.UTC)
	platform := successfulPlatform()
	platform.loginContext.GameplayPolicy = GameplayPolicy{
		Revision:  42,
		ChannelID: 1,
		Candidates: []GameplayPolicyCandidate{
			policyCandidate("canary", "canary.current", "canary.sequence.v1", "canary-endpoint", 7172, []string{"session.single-admission.v1"}, nil),
			policyCandidate("oteryn", 1, "tcp.tls13.protobuf.be32.v1", "native-endpoint", 7173, nativeV1BaseCapabilities, []string{"zz.optional.v1"}),
		},
	}
	sessions := &fakeSessionIssuer{session: Session{Credential: "v2-session", ExpiresAt: now.Add(time.Minute)}}
	service := NewService(platform, sessions)
	service.now = func() time.Time { return now }

	offer := GameplayOffer{
		OfferVersion:   1,
		ClientBuild:    "oteryn-client-test",
		ClientPlatform: "windows-x86_64",
		Candidates: []GameplayOfferCandidate{
			offerCandidate("oteryn", 1, "tcp.tls13.protobuf.be32.v1", append(append([]string(nil), nativeV1BaseCapabilities...), "zz.optional.v1")),
			offerCandidate("canary", "canary.current", "canary.sequence.v1", []string{"session.single-admission.v1"}),
		},
	}

	response, err := service.LoginWithRequest(context.Background(), LoginRequest{ProtocolVersion: 1, GameLoginTicket: "ticket", GameplayOffer: &offer})
	if err != nil {
		t.Fatalf("LoginWithRequest returned error: %v", err)
	}
	if response.GameSessionContractVersion != 2 || response.GameplaySelection == nil || response.GameplaySelection.Family != "canary" {
		t.Fatalf("Gateway did not use authoritative policy order: %#v", response.GameplaySelection)
	}
	if sessions.readyForCalls != 1 || sessions.calls != 1 {
		t.Fatalf("unexpected v2 dependency calls: ready=%d create=%d", sessions.readyForCalls, sessions.calls)
	}
	request := sessions.request
	if request.ContractVersion != 2 || request.SecurityGeneration != 0 || request.ChannelID != 1 || request.WorldPolicyRevision != 42 || request.EndpointID != "canary-endpoint" || !request.SingleAdmission {
		t.Fatalf("incomplete v2 binding: %#v", request)
	}
	if request.Audience != "otheryn-world:1:channel:1:endpoint:canary-endpoint" || request.CharacterBindingMode != "bind_on_first_admission" {
		t.Fatalf("unexpected v2 audience/binding: %#v", request)
	}
}

func TestLoginWithOfferConsumesValidLoginBeforeNoMatchAndDoesNotIssue(t *testing.T) {
	platform := successfulPlatform()
	platform.loginContext.GameplayPolicy = GameplayPolicy{Revision: 2, ChannelID: 1, Candidates: []GameplayPolicyCandidate{
		policyCandidate("oteryn", 1, "tcp.tls13.protobuf.be32.v1", "native", 7173, nativeV1BaseCapabilities, nil),
	}}
	sessions := &fakeSessionIssuer{}
	service := NewService(platform, sessions)
	offer := GameplayOffer{OfferVersion: 1, ClientBuild: "test", ClientPlatform: "linux-x86_64", Candidates: []GameplayOfferCandidate{
		offerCandidate("canary", "canary.current", "canary.sequence.v1", []string{"session.single-admission.v1"}),
	}}

	_, err := service.LoginWithRequest(context.Background(), LoginRequest{ProtocolVersion: 1, GameLoginTicket: "ticket", GameplayOffer: &offer})
	if !errors.Is(err, ErrUnsupportedGameplayPair) {
		t.Fatalf("expected unsupported pair, got %v", err)
	}
	if platform.redeemCalls != 1 || platform.contextCalls != 1 || sessions.readyForCalls != 0 || sessions.calls != 0 {
		t.Fatalf("unexpected no-match calls: platform=%#v sessions=%#v", platform, sessions)
	}
}

func TestLoginWithOfferFailsClosedOnReadinessWithoutTryingAnotherCandidate(t *testing.T) {
	platform := successfulPlatform()
	platform.loginContext.GameplayPolicy = GameplayPolicy{Revision: 3, ChannelID: 1, Candidates: []GameplayPolicyCandidate{
		policyCandidate("canary", "canary.current", "canary.sequence.v1", "first", 7172, []string{"session.single-admission.v1"}, nil),
		policyCandidate("canary", "canary.backup", "canary.sequence.v1", "second", 7174, []string{"session.single-admission.v1"}, nil),
	}}
	sessions := &fakeSessionIssuer{readyForErr: ErrUnavailable}
	service := NewService(platform, sessions)
	offer := GameplayOffer{OfferVersion: 1, ClientBuild: "test", ClientPlatform: "linux-x86_64", Candidates: []GameplayOfferCandidate{
		offerCandidate("canary", "canary.backup", "canary.sequence.v1", []string{"session.single-admission.v1"}),
		offerCandidate("canary", "canary.current", "canary.sequence.v1", []string{"session.single-admission.v1"}),
	}}

	_, err := service.LoginWithRequest(context.Background(), LoginRequest{ProtocolVersion: 1, GameLoginTicket: "ticket", GameplayOffer: &offer})
	if !errors.Is(err, ErrUnavailable) {
		t.Fatalf("expected unavailable, got %v", err)
	}
	if sessions.readyForCalls != 1 || sessions.calls != 0 || sessions.readyRequest.EndpointID != "first" {
		t.Fatalf("Gateway attempted fallback or issuance: %#v", sessions)
	}
}

func TestLoginStopsAfterInvalidTicket(t *testing.T) {
	platform := &fakePlatform{redeemErr: ErrInvalidLogin}
	sessions := &fakeSessionIssuer{}
	service := NewService(platform, sessions)

	_, err := service.Login(context.Background(), "invalid")
	if !errors.Is(err, ErrInvalidLogin) {
		t.Fatalf("expected ErrInvalidLogin, got %v", err)
	}
	if platform.contextCalls != 0 || sessions.calls != 0 {
		t.Fatalf("downstream dependencies were called after invalid ticket")
	}
}

func TestLoginFailsClosedForAmbiguousWorlds(t *testing.T) {
	platform := successfulPlatform()
	platform.loginContext.Worlds = append(platform.loginContext.Worlds, World{ID: 2, Host: "two.test", Port: 7172})
	sessions := &fakeSessionIssuer{}
	service := NewService(platform, sessions)

	_, err := service.Login(context.Background(), "ticket")
	if !errors.Is(err, ErrUnavailable) || sessions.calls != 0 {
		t.Fatalf("expected fail-closed ambiguous world, err=%v calls=%d", err, sessions.calls)
	}
}

func TestLoginFailsClosedForCharacterWorldMismatch(t *testing.T) {
	platform := successfulPlatform()
	platform.loginContext.Characters[0].WorldID = 2
	sessions := &fakeSessionIssuer{}
	service := NewService(platform, sessions)

	_, err := service.Login(context.Background(), "ticket")
	if !errors.Is(err, ErrUnavailable) || sessions.calls != 0 {
		t.Fatalf("expected fail-closed character mismatch, err=%v calls=%d", err, sessions.calls)
	}
}

func TestReadyFailsWhenAnyDependencyIsUnavailable(t *testing.T) {
	platform := &fakePlatform{}
	sessions := &fakeSessionIssuer{readyErr: ErrUnavailable}
	service := NewService(platform, sessions)

	if err := service.Ready(context.Background()); !errors.Is(err, ErrUnavailable) {
		t.Fatalf("expected readiness failure, got %v", err)
	}
}

func successfulPlatform() *fakePlatform {
	return &fakePlatform{
		authorization: Authorization{CanaryAccountID: 1001, SecurityGeneration: 0},
		loginContext: LoginContext{
			Worlds:     []World{{ID: 1, Slug: "oteryn", Name: "Oteryn", Region: "EU", Host: "game.example.test", Port: 7172}},
			Characters: []Character{{ID: 10, Name: "Alpha", Level: 100, Vocation: 4, WorldID: 1}},
		},
	}
}

func offerCandidate(family string, identity any, transport string, capabilities []string) GameplayOfferCandidate {
	sorted := append([]string(nil), capabilities...)
	sort.Strings(sorted)
	candidate := GameplayOfferCandidate{
		Family: family, Transport: transport, Capabilities: sorted,
		SchemaSHA256: schemaHashForCandidate(family),
	}
	if family == "oteryn" {
		version, ok := identity.(int)
		if !ok {
			panic("native test identity must be an integer")
		}
		candidate.NativeProtocolVersion = uint32(version)
		candidate.SchemaRevision = 2
	} else {
		profile, ok := identity.(string)
		if !ok {
			panic("compatibility test identity must be a profile string")
		}
		candidate.Profile = profile
		candidate.SchemaRevision = 1
	}
	return candidate
}

func schemaHashForCandidate(family string) string {
	if family == "oteryn" {
		return canonicalNativeSchemaSHA256
	}
	return strings.Repeat("a", 64)
}

func policyCandidate(family string, identity any, transport, endpoint string, port int, required, optional []string) GameplayPolicyCandidate {
	requiredCopy := append([]string(nil), required...)
	optionalCopy := append([]string(nil), optional...)
	sort.Strings(requiredCopy)
	sort.Strings(optionalCopy)
	candidate := GameplayPolicyCandidate{
		Family: family, Transport: transport,
		SchemaSHA256:         schemaHashForCandidate(family),
		RequiredCapabilities: requiredCopy, OptionalCapabilities: optionalCopy,
		EndpointID: endpoint, Host: "game.example.test", Port: port, TLSServerName: "game.example.test",
	}
	if family == "oteryn" {
		version, ok := identity.(int)
		if !ok {
			panic("native test identity must be an integer")
		}
		candidate.NativeProtocolVersion = uint32(version)
		candidate.SchemaRevision = 2
	} else {
		profile, ok := identity.(string)
		if !ok {
			panic("compatibility test identity must be a profile string")
		}
		candidate.Profile = profile
		candidate.SchemaRevision = 1
	}
	return candidate
}
