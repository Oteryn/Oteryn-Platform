package httpapi

import (
	"bytes"
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"sort"
	"strings"
	"testing"
	"time"

	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/gateway"
)

type testPlatform struct {
	authorization gateway.Authorization
	loginContext  gateway.LoginContext
	redeemErr     error
	contextErr    error
	readyErr      error
	redeemCalls   int
	contextCalls  int
}

func (f *testPlatform) Redeem(context.Context, string) (gateway.Authorization, error) {
	f.redeemCalls++
	return f.authorization, f.redeemErr
}

func (f *testPlatform) LoginContext(context.Context, int64) (gateway.LoginContext, error) {
	f.contextCalls++
	return f.loginContext, f.contextErr
}

func (f *testPlatform) Ready(context.Context) error { return f.readyErr }

type testSessionIssuer struct {
	session       gateway.Session
	err           error
	readyErr      error
	readyForErr   error
	calls         int
	readyForCalls int
	request       gateway.SessionRequest
}

func (f *testSessionIssuer) Create(_ context.Context, request gateway.SessionRequest) (gateway.Session, error) {
	f.calls++
	f.request = request
	return f.session, f.err
}

func (f *testSessionIssuer) Ready(context.Context) error { return f.readyErr }

func (f *testSessionIssuer) ReadyFor(_ context.Context, request gateway.SessionRequest) error {
	f.readyForCalls++
	f.request = request
	return f.readyForErr
}

func TestLoginSuccessDoesNotLogCredentialsAndIsNotCacheable(t *testing.T) {
	now := time.Now().UTC()
	platform := legacyTestPlatform()
	sessions := &testSessionIssuer{session: gateway.Session{Credential: "session-secret-never-log", ExpiresAt: now.Add(time.Minute)}}
	service := gateway.NewService(platform, sessions)
	var logs bytes.Buffer
	server := NewServer(service, "test-version", slog.New(slog.NewJSONHandler(&logs, nil)))

	body := `{"protocol_version":1,"game_login_ticket":"ticket-secret-never-log"}`
	request := httptest.NewRequest(http.MethodPost, "/v1/login", strings.NewReader(body))
	request.Header.Set("X-Request-ID", "request-123")
	response := httptest.NewRecorder()
	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d body=%s", response.Code, response.Body.String())
	}
	assertSensitiveResponseNoCache(t, response)

	var payload gateway.LoginResponse
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if payload.Session.Credential != "session-secret-never-log" || payload.GameplaySelection != nil {
		t.Fatalf("unexpected legacy session response: %#v", payload)
	}
	logText := logs.String()
	if strings.Contains(logText, "ticket-secret-never-log") || strings.Contains(logText, "session-secret-never-log") {
		t.Fatalf("credential leaked to logs: %s", logText)
	}
	if !strings.Contains(logText, "request-123") || !strings.Contains(logText, `"path":"/v1/login"`) {
		t.Fatalf("bounded request metadata missing from logs: %s", logText)
	}
}

func TestLoginAcceptsBoundedOfferAndReturnsDistinctSelectionFields(t *testing.T) {
	capabilities := nativeCapabilities()
	platform := legacyTestPlatform()
	platform.loginContext.GameplayPolicy = gateway.GameplayPolicy{
		Revision:  9,
		ChannelID: 1,
		Candidates: []gateway.GameplayPolicyCandidate{{
			Family: "oteryn", NativeProtocolVersion: 1, Transport: "tcp.tls13.protobuf.be32.v1",
			SchemaRevision: 2, SchemaSHA256: "9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9", RequiredCapabilities: capabilities,
			EndpointID: "native-eu-1", Host: "native.example.test", Port: 7173, TLSServerName: "native.example.test",
		}},
	}
	sessions := &testSessionIssuer{session: gateway.Session{Credential: "v2-session", ExpiresAt: time.Now().UTC().Add(time.Minute)}}
	server := NewServer(gateway.NewService(platform, sessions), "test", slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))

	body, err := json.Marshal(gateway.LoginRequest{
		ProtocolVersion: 1,
		GameLoginTicket: "ticket",
		GameplayOffer: &gateway.GameplayOffer{
			OfferVersion: 1, ClientBuild: "oteryn-client-test", ClientPlatform: "windows-x86_64",
			Candidates: []gateway.GameplayOfferCandidate{{
				Family: "oteryn", NativeProtocolVersion: 1, Transport: "tcp.tls13.protobuf.be32.v1",
				SchemaRevision: 2, SchemaSHA256: "9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9", Capabilities: capabilities,
			}},
		},
	})
	if err != nil {
		t.Fatalf("encode request: %v", err)
	}

	response := httptest.NewRecorder()
	server.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, "/v1/login", bytes.NewReader(body)))
	if response.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d body=%s", response.Code, response.Body.String())
	}
	var payload gateway.LoginResponse
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if payload.ProtocolVersion != 1 || payload.GameSessionContractVersion != 2 || payload.LoginAttemptID == "" || payload.GameplaySelection == nil {
		t.Fatalf("missing distinct version/selection fields: %#v", payload)
	}
	if sessions.readyForCalls != 1 || sessions.calls != 1 || sessions.request.EndpointID != "native-eu-1" {
		t.Fatalf("unexpected readiness/session calls: %#v", sessions)
	}
}

func TestLoginRejectsInvalidShapeBeforeDependencies(t *testing.T) {
	platform := legacyTestPlatform()
	sessions := &testSessionIssuer{}
	server := NewServer(gateway.NewService(platform, sessions), "test", slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))

	tests := []struct {
		name string
		url  string
		body string
	}{
		{name: "unknown field", url: "/v1/login", body: `{"protocol_version":1,"game_login_ticket":"ticket","password":"secret"}`},
		{name: "duplicate top-level key", url: "/v1/login", body: `{"protocol_version":1,"protocol_version":1,"game_login_ticket":"ticket"}`},
		{name: "duplicate nested key", url: "/v1/login", body: `{"protocol_version":1,"game_login_ticket":"ticket","gameplay_offer":{"offer_version":1,"offer_version":1,"client_build":"test","client_platform":"linux","candidates":[]}}`},
		{name: "zero candidates", url: "/v1/login", body: `{"protocol_version":1,"game_login_ticket":"ticket","gameplay_offer":{"offer_version":1,"client_build":"test","client_platform":"linux","candidates":[]}}`},
		{name: "unsorted capabilities", url: "/v1/login", body: `{"protocol_version":1,"game_login_ticket":"ticket","gameplay_offer":{"offer_version":1,"client_build":"test","client_platform":"linux","candidates":[{"family":"canary","nativeProtocolVersion":"canary.current","transport":"canary.sequence.v1","schema_revision":2,"schema_sha256":"` + strings.Repeat("a", 64) + `","capabilities":["z.v1","a.v1"]}]}}`},
		{name: "query", url: "/v1/login?ticket=secret", body: `{"protocol_version":1,"game_login_ticket":"ticket"}`},
		{name: "oversized", url: "/v1/login", body: strings.Repeat(" ", extendedLoginRequestLimit+1)},
		{name: "legacy remains 4KiB bounded", url: "/v1/login", body: `{"protocol_version":1,"game_login_ticket":"ticket"}` + strings.Repeat(" ", legacyLoginRequestLimit)},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			response := httptest.NewRecorder()
			server.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, test.url, strings.NewReader(test.body)))
			if response.Code != http.StatusBadRequest {
				t.Fatalf("expected 400, got %d body=%s", response.Code, response.Body.String())
			}
			assertSensitiveResponseNoCache(t, response)
		})
	}
	if platform.redeemCalls != 0 || platform.contextCalls != 0 || sessions.calls != 0 || sessions.readyForCalls != 0 {
		t.Fatalf("invalid requests reached dependencies: platform=%#v sessions=%#v", platform, sessions)
	}
}

func TestLoginMapsBoundedErrors(t *testing.T) {
	for _, test := range []struct {
		name   string
		err    error
		status int
		body   string
	}{
		{name: "invalid", err: gateway.ErrInvalidLogin, status: http.StatusUnauthorized, body: "invalid_login"},
		{name: "outage", err: gateway.ErrUnavailable, status: http.StatusServiceUnavailable, body: "login_unavailable"},
	} {
		t.Run(test.name, func(t *testing.T) {
			service := gateway.NewService(&testPlatform{redeemErr: test.err}, &testSessionIssuer{})
			server := NewServer(service, "test", slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))
			response := httptest.NewRecorder()
			server.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, "/v1/login", strings.NewReader(`{"protocol_version":1,"game_login_ticket":"ticket"}`)))
			if response.Code != test.status || !strings.Contains(response.Body.String(), test.body) {
				t.Fatalf("unexpected response: status=%d body=%s", response.Code, response.Body.String())
			}
			assertSensitiveResponseNoCache(t, response)
		})
	}
}

func TestLoginMapsNoMatchToConflictWithoutPolicyDisclosure(t *testing.T) {
	platform := legacyTestPlatform()
	platform.loginContext.GameplayPolicy = gateway.GameplayPolicy{Revision: 1, ChannelID: 1}
	server := NewServer(gateway.NewService(platform, &testSessionIssuer{}), "test", slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))
	body := `{"protocol_version":1,"game_login_ticket":"ticket","gameplay_offer":{"offer_version":1,"client_build":"test","client_platform":"linux","candidates":[{"family":"canary","nativeProtocolVersion":"canary.current","transport":"canary.sequence.v1","schema_revision":2,"schema_sha256":"` + strings.Repeat("a", 64) + `","capabilities":[]}]}}`
	response := httptest.NewRecorder()
	server.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, "/v1/login", strings.NewReader(body)))
	if response.Code != http.StatusConflict || response.Body.String() != "{\"error\":\"unsupported_gameplay_pair\"}\n" {
		t.Fatalf("unexpected no-match response: %d %s", response.Code, response.Body.String())
	}
}

func TestHealthAndReadinessAreSeparate(t *testing.T) {
	service := gateway.NewService(&testPlatform{readyErr: gateway.ErrUnavailable}, &testSessionIssuer{})
	server := NewServer(service, "v1.2.3", slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))

	health := httptest.NewRecorder()
	server.Handler().ServeHTTP(health, httptest.NewRequest(http.MethodGet, "/health", nil))
	if health.Code != http.StatusOK {
		t.Fatalf("health should remain process-local, got %d", health.Code)
	}

	ready := httptest.NewRecorder()
	server.Handler().ServeHTTP(ready, httptest.NewRequest(http.MethodGet, "/ready", nil))
	if ready.Code != http.StatusServiceUnavailable {
		t.Fatalf("readiness should fail on dependency outage, got %d", ready.Code)
	}

	version := httptest.NewRecorder()
	server.Handler().ServeHTTP(version, httptest.NewRequest(http.MethodGet, "/version", nil))
	if version.Code != http.StatusOK || !strings.Contains(version.Body.String(), "v1.2.3") {
		t.Fatalf("unexpected version response: %d %s", version.Code, version.Body.String())
	}
}

func legacyTestPlatform() *testPlatform {
	return &testPlatform{
		authorization: gateway.Authorization{CanaryAccountID: 1001, SecurityGeneration: 7},
		loginContext: gateway.LoginContext{
			Worlds:     []gateway.World{{ID: 1, Slug: "oteryn", Name: "Oteryn", Region: "EU", Host: "game.example.test", Port: 7172}},
			Characters: []gateway.Character{{ID: 10, Name: "Alpha", Level: 100, Vocation: 4, WorldID: 1}},
		},
	}
}

func nativeCapabilities() []string {
	capabilities := []string{
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
	sort.Strings(capabilities)
	return capabilities
}

func assertSensitiveResponseNoCache(t *testing.T, response *httptest.ResponseRecorder) {
	t.Helper()

	if got := response.Header().Get("Cache-Control"); got != "no-store, no-cache, must-revalidate, private" {
		t.Fatalf("unexpected Cache-Control header: %q", got)
	}
	if got := response.Header().Get("Pragma"); got != "no-cache" {
		t.Fatalf("unexpected Pragma header: %q", got)
	}
	if got := response.Header().Get("Expires"); got != "0" {
		t.Fatalf("unexpected Expires header: %q", got)
	}
}
