package e2e

import (
	"bytes"
	"encoding/json"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"

	"strings"
	"sync"
	"testing"
	"time"
)

// TestNativeLoginJourneyForwardsToTheIssuerIdempotently drives the real Gateway server and
// issuer client against an issuer double that keeps the §6.2 attempt record: the first request
// commits a grant, a retry of the same request returns the byte-identical token with a lower
// validity, and the same attempt_ref with another ticket conflicts. No other Platform endpoint is
// ever called on the native branch.
func TestNativeLoginJourneyForwardsToTheIssuerIdempotently(t *testing.T) {
	const serviceToken = "synthetic-service-token"
	const attemptRef = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b"
	const grantToken = "eyJhbGciOiJFZDI1NTE5In0.eyJhdHRlbXB0In0.c2lnbmF0dXJl"

	var lock sync.Mutex
	committed := map[string]string{} // attempt_ref -> forwarded body
	issuerCalls := 0
	validFor := 20
	otherCalls := 0

	platformServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		lock.Lock()
		defer lock.Unlock()
		if r.Header.Get("Authorization") != "Bearer "+serviceToken {
			w.WriteHeader(http.StatusUnauthorized)
			return
		}
		if r.URL.Path != "/internal/v1/game-auth/native-admissions" {
			otherCalls++
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
	var logs bytes.Buffer
	binary := os.Getenv("GATEWAY_RUST_BINARY")
	if binary == "" {
		t.Skip("GATEWAY_RUST_BINARY is unset; run against the built Rust role")
	}
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	address := listener.Addr().String()
	_ = listener.Close()
	tokenFile := filepath.Join(t.TempDir(), "service-token")
	if err := os.WriteFile(tokenFile, []byte(serviceToken+"\n"), 0600); err != nil {
		t.Fatal(err)
	}
	command := exec.Command(binary, "gateway")
	command.Env = append(os.Environ(), "OTERYN_PLATFORM_BASE_URL="+platformServer.URL,
		"OTERYN_PLATFORM_SERVICE_TOKEN_FILE="+tokenFile, "GATEWAY_LISTEN_ADDR="+address,
		"GATEWAY_NATIVE_LOGIN_ENABLED=true", "OTERYN_DATABASE_URL=invalid", "DATABASE_URL=invalid")
	command.Stdout = &logs
	command.Stderr = &logs
	if err := command.Start(); err != nil {
		t.Fatal(err)
	}
	stopped := false
	defer func() {
		if !stopped {
			_ = command.Process.Kill()
			_ = command.Wait()
		}
	}()
	gatewayURL := "http://" + address
	ready := false
	for i := 0; i < 100; i++ {
		response, err := http.Get(gatewayURL + "/health")
		if err == nil {
			_ = response.Body.Close()
			ready = response.StatusCode == 200
			if ready {
				break
			}
		}
		time.Sleep(10 * time.Millisecond)
	}
	if !ready {
		t.Fatal("Rust gateway role did not start")
	}

	login := func(ticket string) (int, map[string]any) {
		body := `{"protocol_version":2,"game_login_ticket":"` + ticket + `","attempt_ref":"` + attemptRef +
			`","character_id":"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c","channel_id":null,"offer":{"client_build":"0.1.0",` +
			`"client_platform":"windows","transports":[{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}]}}`
		response, err := http.Post(gatewayURL+"/v1/login", "application/json", strings.NewReader(body))
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

	_ = command.Process.Kill()
	_ = command.Wait()
	stopped = true
	lock.Lock()
	defer lock.Unlock()
	if issuerCalls != 3 || otherCalls != 0 {
		t.Fatalf("expected three issuer calls and no other Platform call, got issuer=%d other=%d", issuerCalls, otherCalls)
	}
	for _, secret := range []string{"native-ticket-secret", "another-ticket", grantToken, serviceToken} {
		if strings.Contains(logs.String(), secret) {
			t.Fatalf("logs contain a secret: %s", logs.String())
		}
	}
}
