package e2e

import (
	"bytes"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/gateway"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/httpapi"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/platform"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/session"
)

func TestNativeProducerJourneyBindsOneSelectionAndIssuesOnce(t *testing.T) {
	const serviceToken = "synthetic-service-token"
	const schemaHash = "9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9"
	capabilities := nativeCapabilities()

	var lock sync.Mutex
	redeemCalls := 0
	contextCalls := 0
	readinessCalls := 0
	issueCalls := 0

	platformServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Authorization") != "Bearer "+serviceToken {
			t.Fatalf("missing Platform service authentication")
		}
		lock.Lock()
		defer lock.Unlock()
		switch r.URL.Path {
		case "/internal/v1/game-auth/tickets/redeem":
			redeemCalls++
			_ = json.NewEncoder(w).Encode(map[string]any{
				"protocol_version": 1,
				"authorization": map[string]any{
					"canary_account_id":   1001,
					"security_generation": 0,
					"redeemed_at":         "2026-08-04T16:00:00Z",
				},
			})
		case "/internal/v1/game-auth/accounts/1001/login-context":
			contextCalls++
			_ = json.NewEncoder(w).Encode(map[string]any{
				"protocol_version": 1,
				"worlds":           []map[string]any{{"id": 1, "slug": "test", "name": "Test", "region": "TEST", "host": "legacy.example.test", "port": 7172}},
				"characters":       []map[string]any{{"id": 10, "name": "Alpha", "level": 100, "vocation": 4, "world_id": 1}},
				"gameplay_policy": map[string]any{
					"revision":   17,
					"channel_id": 1,
					"candidates": []map[string]any{{
						"family":                  "oteryn",
						"native_protocol_version": 1,
						"transport":               "tcp.tls13.protobuf.be32.v1",
						"schema_revision":         2,
						"schema_sha256":           schemaHash,
						"required_capabilities":   capabilities,
						"optional_capabilities":   []string{},
						"endpoint_id":             "native-test-1",
						"host":                    "native.example.test",
						"port":                    7173,
						"tls_server_name":         "native.example.test",
					}},
				},
			})
		default:
			t.Fatalf("unexpected Platform path: %s", r.URL.Path)
		}
	}))
	defer platformServer.Close()

	sessionServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Authorization") != "Bearer "+serviceToken {
			t.Fatalf("missing session service authentication")
		}
		var payload map[string]any
		if err := json.NewDecoder(r.Body).Decode(&payload); err != nil {
			t.Fatalf("decode session payload: %v", err)
		}
		if payload["contract_version"] != float64(2) || payload["endpoint_id"] != "native-test-1" || payload["identity_security_generation"] != float64(0) {
			t.Fatalf("incomplete v2 session binding: %#v", payload)
		}

		lock.Lock()
		defer lock.Unlock()
		switch r.URL.Path {
		case "/internal/v2/game-sessions/readiness":
			readinessCalls++
			_ = json.NewEncoder(w).Encode(map[string]any{
				"contract_version":         payload["contract_version"],
				"ready":                    true,
				"world_id":                 payload["world_id"],
				"channel_id":               payload["channel_id"],
				"world_policy_revision":    payload["world_policy_revision"],
				"endpoint_id":              payload["endpoint_id"],
				"audience":                 payload["audience"],
				"family":                   payload["family"],
				"native_protocol_version":  payload["native_protocol_version"],
				"transport":                payload["transport"],
				"schema_revision":          payload["schema_revision"],
				"schema_sha256":            payload["schema_sha256"],
				"capabilities":             payload["capabilities"],
				"capability_digest_sha256": payload["capability_digest_sha256"],
			})
		case "/internal/v2/game-sessions":
			issueCalls++
			_ = json.NewEncoder(w).Encode(map[string]any{
				"contract_version": 2,
				"session": map[string]any{
					"credential": "synthetic-v2-credential",
					"expires_at": time.Now().UTC().Add(time.Minute).Format(time.RFC3339),
				},
			})
		default:
			t.Fatalf("unexpected session path: %s", r.URL.Path)
		}
	}))
	defer sessionServer.Close()

	httpClient := &http.Client{Timeout: 2 * time.Second}
	service := gateway.NewService(
		platform.NewClient(platformServer.URL, serviceToken, httpClient),
		session.NewClient(sessionServer.URL, serviceToken, httpClient),
	)
	api := httpapi.NewServer(service, "e2e", slog.New(slog.NewTextHandler(&bytes.Buffer{}, nil)))

	body, err := json.Marshal(gateway.LoginRequest{
		ProtocolVersion: 1,
		GameLoginTicket: "synthetic-one-time-ticket",
		GameplayOffer: &gateway.GameplayOffer{
			OfferVersion:   1,
			ClientBuild:    "oteryn-client-e2e",
			ClientPlatform: "linux-x86_64",
			Candidates: []gateway.GameplayOfferCandidate{{
				Family:                "oteryn",
				NativeProtocolVersion: 1,
				Transport:             "tcp.tls13.protobuf.be32.v1",
				SchemaRevision:        2,
				SchemaSHA256:          schemaHash,
				Capabilities:          capabilities,
			}},
		},
	})
	if err != nil {
		t.Fatalf("encode login request: %v", err)
	}

	response := httptest.NewRecorder()
	api.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, "/v1/login", bytes.NewReader(body)))
	if response.Code != http.StatusOK {
		t.Fatalf("producer journey failed: %d %s", response.Code, response.Body.String())
	}

	var login gateway.LoginResponse
	if err := json.Unmarshal(response.Body.Bytes(), &login); err != nil {
		t.Fatalf("decode Gateway response: %v", err)
	}
	if login.GameSessionContractVersion != 2 || login.GameplaySelection == nil || login.GameplaySelection.Host != "native.example.test" || login.Session.Credential != "synthetic-v2-credential" {
		t.Fatalf("unexpected Gateway response: %#v", login)
	}
	if redeemCalls != 1 || contextCalls != 1 || readinessCalls != 1 || issueCalls != 1 {
		t.Fatalf("unexpected producer call counts: redeem=%d context=%d readiness=%d issue=%d", redeemCalls, contextCalls, readinessCalls, issueCalls)
	}
}

func nativeCapabilities() []string {
	return strings.Fields(`actions.command-result.v1
chat.semantic.v1
combat.server-authoritative.v1
inventory.server-authoritative.v1
ordering.server-sequence.v1
reconciliation.movement.v1
session.single-admission.v1
state.revision.v1
state.snapshot-delta.v1`)
}
