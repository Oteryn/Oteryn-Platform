package httpapi

import (
	"bytes"
	"context"
	"errors"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

type testReadiness struct{ err error }

func (r testReadiness) Ready(context.Context) error { return r.err }

type recordingNativeHandler struct {
	bodies []string
}

func (h *recordingNativeHandler) Serve(w http.ResponseWriter, _ *http.Request, body []byte) {
	h.bodies = append(h.bodies, string(body))
	w.WriteHeader(http.StatusTeapot)
}

const nativeLoginBody = `{"protocol_version":2,"game_login_ticket":"native-ticket","attempt_ref":"0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b","character_id":"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c","channel_id":null,"offer":{"client_build":"0.1.0","client_platform":"windows","transports":[{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}]}}`

func newTestServer(readyErr error) *Server {
	return NewServer(testReadiness{err: readyErr}, "test", slog.New(slog.NewJSONHandler(&bytes.Buffer{}, nil)))
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

func TestHealthAndReadinessAreSeparate(t *testing.T) {
	server := NewServer(testReadiness{err: errors.New("platform down")}, "v1.2.3", slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))

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

func TestReadinessSucceedsWhenPlatformIsReachable(t *testing.T) {
	ready := httptest.NewRecorder()
	newTestServer(nil).Handler().ServeHTTP(ready, httptest.NewRequest(http.MethodGet, "/ready", nil))
	if ready.Code != http.StatusOK {
		t.Fatalf("expected ready, got %d", ready.Code)
	}
}

func TestLoginIsDefaultOffAndRefusesEveryBody(t *testing.T) {
	server := newTestServer(nil)

	for _, body := range []string{nativeLoginBody, `{"protocol_version":1,"game_login_ticket":"ticket"}`, ``} {
		response := httptest.NewRecorder()
		server.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, "/v1/login", strings.NewReader(body)))
		if response.Code != http.StatusBadRequest || strings.TrimSpace(response.Body.String()) != `{"error":"invalid_request"}` {
			t.Fatalf("expected invalid_request while native login is disabled, got %d %s", response.Code, response.Body.String())
		}
		assertSensitiveResponseNoCache(t, response)
	}
}

func TestLoginRoutesOnlyProtocolVersionTwoToTheNativeBranch(t *testing.T) {
	server := newTestServer(nil)
	native := &recordingNativeHandler{}
	server.EnableNativeLogin(native)

	for _, body := range []string{nativeLoginBody, `{"protocol_version":2,"protocol_version":2}`} {
		response := httptest.NewRecorder()
		server.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, "/v1/login", strings.NewReader(body)))
		if response.Code != http.StatusTeapot {
			t.Fatalf("protocol_version 2 must reach the native branch, got %d", response.Code)
		}
		assertSensitiveResponseNoCache(t, response)
	}

	for _, body := range []string{
		`{"protocol_version":1,"game_login_ticket":"ticket"}`,
		`{"game_login_ticket":"ticket"}`,
		`{"protocol_version":"2"}`,
		`not json`,
		``,
	} {
		response := httptest.NewRecorder()
		server.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, "/v1/login", strings.NewReader(body)))
		if response.Code != http.StatusBadRequest || strings.TrimSpace(response.Body.String()) != `{"error":"invalid_request"}` {
			t.Fatalf("body %q must be refused as invalid_request, got %d %s", body, response.Code, response.Body.String())
		}
	}
	if len(native.bodies) != 2 {
		t.Fatalf("only the two protocol_version 2 bodies may reach the native branch, got %d", len(native.bodies))
	}
}

func TestLoginRejectsQueryStringAndOversizedBodies(t *testing.T) {
	server := newTestServer(nil)
	native := &recordingNativeHandler{}
	server.EnableNativeLogin(native)

	query := httptest.NewRecorder()
	server.Handler().ServeHTTP(query, httptest.NewRequest(http.MethodPost, "/v1/login?x=1", strings.NewReader(nativeLoginBody)))
	if query.Code != http.StatusBadRequest {
		t.Fatalf("a query string must be refused, got %d", query.Code)
	}

	oversized := httptest.NewRecorder()
	server.Handler().ServeHTTP(oversized, httptest.NewRequest(http.MethodPost, "/v1/login", strings.NewReader(strings.Repeat(" ", loginRequestLimit+1))))
	if oversized.Code != http.StatusBadRequest {
		t.Fatalf("an oversized body must be refused, got %d", oversized.Code)
	}
	if len(native.bodies) != 0 {
		t.Fatal("refused requests must not reach the native branch")
	}
}
