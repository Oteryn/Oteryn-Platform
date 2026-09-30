package e2e

import (
	"bytes"
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/gateway"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/httpapi"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/nativelogin"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/platform"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/session"
)

// TestNativeLoginJourneyForwardsToTheIssuerIdempotently drives the real Gateway server and
// issuer client against an issuer double that keeps the §6.2 attempt record: the first request
// commits a grant, a retry of the same request returns the byte-identical token with a lower
// validity, and the same attempt_ref with another ticket conflicts. No Canary dependency is ever
// called on the native branch.
func TestNativeLoginJourneyForwardsToTheIssuerIdempotently(t *testing.T) {
	const serviceToken = "synthetic-service-token"
	const attemptRef = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b"
	const grantToken = "eyJhbGciOiJFZDI1NTE5In0.eyJhdHRlbXB0In0.c2lnbmF0dXJl"

	var lock sync.Mutex
	committed := map[string]string{} // attempt_ref -> forwarded body
	issuerCalls := 0
	validFor := 20
	canaryCalls := 0

	platformServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		lock.Lock()
		defer lock.Unlock()
		if r.Header.Get("Authorization") != "Bearer "+serviceToken {
			w.WriteHeader(http.StatusUnauthorized)
			return
		}
		if r.URL.Path != "/internal/v1/game-auth/native-admissions" {
			canaryCalls++
			w.WriteHeader(http.StatusNotFound)
			return
		}
		issuerCalls++
		body, _ := io.ReadAll(r.Body)
		var request struct {
			AttemptRef string `json:"attempt_ref"`
		}
		_ = json.Unmarshal(body, &request)
		w.Header().Set("Content-Type", "application/json")
		if previous, exists := committed[request.AttemptRef]; exists && previous != string(body) {
			w.WriteHeader(http.StatusConflict)
			_, _ = io.WriteString(w, `{"protocol_version":2,"error":{"code":"NATIVE_LOGIN_ATTEMPT_CONFLICT","public_class":"RETRY_LOGIN"},"attempt_ref":"`+request.AttemptRef+`"}`)
			return
		}
		committed[request.AttemptRef] = string(body)
		_ = json.NewEncoder(w).Encode(map[string]any{
			"protocol_version": 2,
			"attempt_ref":      request.AttemptRef,
			"world_id":         "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7e",
			"channel_id":       "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7d",
			"endpoint": map[string]any{
				"host": "game-eu1.example.test", "port": 7172, "tls_server_name": "game-eu1.example.test",
				"alpn": "oteryn-game/1", "protocol_major": 1, "transport_profile": 1,
			},
			"grant": map[string]any{"profile": "oteryn-pre-admission-v1", "token": grantToken, "valid_for_seconds": validFor},
		})
		validFor -= 3
	}))
	defer platformServer.Close()
	sessionServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		lock.Lock()
		canaryCalls++
		lock.Unlock()
		w.WriteHeader(http.StatusInternalServerError)
	}))
	defer sessionServer.Close()

	httpClient := &http.Client{Timeout: 2 * time.Second}
	service := gateway.NewService(
		platform.NewClient(platformServer.URL, serviceToken, httpClient),
		session.NewClient(sessionServer.URL, "session-token", httpClient),
	)
	var logs bytes.Buffer
	logger := slog.New(slog.NewJSONHandler(&logs, nil))
	api := httpapi.NewServer(service, "test", logger)
	api.EnableNativeLogin(nativelogin.NewHandler(nativelogin.NewIssuerClient(platformServer.URL, serviceToken, 2*time.Second, nil), logger))
	gatewayServer := httptest.NewServer(api.Handler())
	defer gatewayServer.Close()

	login := func(ticket string) (int, map[string]any) {
		body := `{"protocol_version":2,"game_login_ticket":"` + ticket + `","attempt_ref":"` + attemptRef +
			`","character_id":"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c","channel_id":null,"offer":{"client_build":"0.1.0",` +
			`"client_platform":"windows","transports":[{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}]}}`
		response, err := http.Post(gatewayServer.URL+"/v1/login", "application/json", strings.NewReader(body))
		if err != nil {
			t.Fatalf("login: %v", err)
		}
		defer response.Body.Close()
		if response.Header.Get("Cache-Control") == "" || !strings.Contains(response.Header.Get("Cache-Control"), "no-store") {
			t.Fatalf("native response must be no-store")
		}
		var decoded map[string]any
		if err := json.NewDecoder(response.Body).Decode(&decoded); err != nil {
			t.Fatalf("decode: %v", err)
		}
		return response.StatusCode, decoded
	}

	status, first := login("native-ticket-secret")
	if status != http.StatusOK {
		t.Fatalf("expected 200, got %d %v", status, first)
	}
	status, retry := login("native-ticket-secret")
	if status != http.StatusOK {
		t.Fatalf("expected retry 200, got %d %v", status, retry)
	}
	firstGrant := first["grant"].(map[string]any)
	retryGrant := retry["grant"].(map[string]any)
	if firstGrant["token"] != grantToken || retryGrant["token"] != grantToken || retryGrant["valid_for_seconds"].(float64) >= firstGrant["valid_for_seconds"].(float64) {
		t.Fatalf("a retry must return the same token with lower validity: %v / %v", firstGrant, retryGrant)
	}

	status, conflict := login("another-ticket")
	errorBody, _ := conflict["error"].(map[string]any)
	if status != http.StatusConflict || errorBody["code"] != "NATIVE_LOGIN_ATTEMPT_CONFLICT" || conflict["attempt_ref"] != attemptRef {
		t.Fatalf("expected attempt conflict, got %d %v", status, conflict)
	}

	lock.Lock()
	defer lock.Unlock()
	if issuerCalls != 3 || canaryCalls != 0 {
		t.Fatalf("expected three issuer calls and no Canary call, got issuer=%d canary=%d", issuerCalls, canaryCalls)
	}
	for _, secret := range []string{"native-ticket-secret", "another-ticket", grantToken, serviceToken} {
		if strings.Contains(logs.String(), secret) {
			t.Fatalf("logs contain a secret: %s", logs.String())
		}
	}
}
