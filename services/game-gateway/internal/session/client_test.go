package session

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"reflect"
	"strings"
	"testing"
	"time"

	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/gateway"
)

func TestCreatePreservesV1ServiceContract(t *testing.T) {
	expiresAt := time.Now().UTC().Add(time.Minute).Truncate(time.Second)
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost || r.URL.Path != "/internal/v1/game-sessions" || r.URL.RawQuery != "" {
			t.Fatalf("unexpected request: %s %s", r.Method, r.URL.String())
		}
		assertServiceAuthentication(t, r)
		var payload struct {
			ProtocolVersion int    `json:"protocol_version"`
			CanaryAccountID int64  `json:"canary_account_id"`
			WorldID         int64  `json:"world_id"`
			LoginAttemptID  string `json:"login_attempt_id"`
		}
		if err := json.NewDecoder(r.Body).Decode(&payload); err != nil {
			t.Fatalf("decode request: %v", err)
		}
		if payload.ProtocolVersion != 1 || payload.CanaryAccountID != 1001 || payload.WorldID != 1 || payload.LoginAttemptID != "attempt-123" {
			t.Fatalf("unexpected session payload: %#v", payload)
		}
		_ = json.NewEncoder(w).Encode(map[string]any{
			"protocol_version": 1,
			"session":          map[string]any{"credential": "session-secret", "expires_at": expiresAt},
		})
	}))
	defer server.Close()

	client := NewClient(server.URL, "session-service-token", server.Client())
	created, err := client.Create(context.Background(), gateway.SessionRequest{
		ContractVersion: 1, CanaryAccountID: 1001, WorldID: 1, LoginAttemptID: "attempt-123",
	})
	if err != nil {
		t.Fatalf("Create returned error: %v", err)
	}
	if created.Credential != "session-secret" || !created.ExpiresAt.Equal(expiresAt) {
		t.Fatalf("unexpected session: %#v", created)
	}
}

func TestReadyForAndCreateV2UseExactBoundContract(t *testing.T) {
	expiresAt := time.Now().UTC().Add(time.Minute).Truncate(time.Second)
	request := validV2Request()
	calls := []string{}
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assertServiceAuthentication(t, r)
		calls = append(calls, r.URL.Path)
		var payload v2RequestPayload
		if err := json.NewDecoder(r.Body).Decode(&payload); err != nil {
			t.Fatalf("decode v2 payload: %v", err)
		}
		assertV2Payload(t, payload, request)

		switch r.URL.Path {
		case "/internal/v2/game-sessions/readiness":
			_ = json.NewEncoder(w).Encode(map[string]any{
				"contract_version":         2,
				"ready":                    true,
				"world_id":                 payload.WorldID,
				"channel_id":               payload.ChannelID,
				"world_policy_revision":    payload.WorldPolicyRevision,
				"endpoint_id":              payload.EndpointID,
				"audience":                 payload.Audience,
				"family":                   payload.Family,
				"native_protocol_version":  payload.NativeProtocolVersion,
				"transport":                payload.Transport,
				"schema_revision":          payload.SchemaRevision,
				"schema_sha256":            payload.SchemaSHA256,
				"capabilities":             payload.Capabilities,
				"capability_digest_sha256": payload.CapabilityDigestSHA256,
			})
		case "/internal/v2/game-sessions":
			_ = json.NewEncoder(w).Encode(map[string]any{
				"contract_version": 2,
				"session":          map[string]any{"credential": "v2-session-secret", "expires_at": expiresAt},
			})
		default:
			t.Fatalf("unexpected v2 path: %s", r.URL.Path)
		}
	}))
	defer server.Close()

	client := NewClient(server.URL, "session-service-token", server.Client())
	if err := client.ReadyFor(context.Background(), request); err != nil {
		t.Fatalf("ReadyFor returned error: %v", err)
	}
	created, err := client.Create(context.Background(), request)
	if err != nil {
		t.Fatalf("Create v2 returned error: %v", err)
	}
	if created.Credential != "v2-session-secret" || !created.ExpiresAt.Equal(expiresAt) {
		t.Fatalf("unexpected v2 session: %#v", created)
	}
	if !reflect.DeepEqual(calls, []string{"/internal/v2/game-sessions/readiness", "/internal/v2/game-sessions"}) {
		t.Fatalf("unexpected v2 call sequence: %#v", calls)
	}
}

func TestReadyForAndCreateV2PreserveCanaryProfile(t *testing.T) {
	expiresAt := time.Now().UTC().Add(time.Minute).Truncate(time.Second)
	request := validCanaryV2Request()
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		assertServiceAuthentication(t, r)
		var payload v2RequestPayload
		if err := json.NewDecoder(r.Body).Decode(&payload); err != nil {
			t.Fatalf("decode Canary v2 payload: %v", err)
		}
		assertV2Payload(t, payload, request)
		if payload.Profile != "canary.current" || payload.NativeProtocolVersion != 0 {
			t.Fatalf("Canary family identity was not preserved: %#v", payload)
		}

		switch r.URL.Path {
		case "/internal/v2/game-sessions/readiness":
			_ = json.NewEncoder(w).Encode(map[string]any{
				"contract_version":         2,
				"ready":                    true,
				"world_id":                 payload.WorldID,
				"channel_id":               payload.ChannelID,
				"world_policy_revision":    payload.WorldPolicyRevision,
				"endpoint_id":              payload.EndpointID,
				"audience":                 payload.Audience,
				"family":                   payload.Family,
				"profile":                  payload.Profile,
				"transport":                payload.Transport,
				"schema_revision":          payload.SchemaRevision,
				"schema_sha256":            payload.SchemaSHA256,
				"capabilities":             payload.Capabilities,
				"capability_digest_sha256": payload.CapabilityDigestSHA256,
			})
		case "/internal/v2/game-sessions":
			_ = json.NewEncoder(w).Encode(map[string]any{
				"contract_version": 2,
				"session":          map[string]any{"credential": "canary-session-secret", "expires_at": expiresAt},
			})
		default:
			t.Fatalf("unexpected Canary v2 path: %s", r.URL.Path)
		}
	}))
	defer server.Close()

	client := NewClient(server.URL, "session-service-token", server.Client())
	if err := client.ReadyFor(context.Background(), request); err != nil {
		t.Fatalf("Canary ReadyFor returned error: %v", err)
	}
	created, err := client.Create(context.Background(), request)
	if err != nil {
		t.Fatalf("Create Canary v2 returned error: %v", err)
	}
	if created.Credential != "canary-session-secret" || !created.ExpiresAt.Equal(expiresAt) {
		t.Fatalf("unexpected Canary v2 session: %#v", created)
	}
}

func TestV2PayloadSerializesExclusiveFamilyIdentity(t *testing.T) {
	tests := []struct {
		name       string
		request    gateway.SessionRequest
		presentKey string
		absentKey  string
	}{
		{
			name:       "native version only",
			request:    validV2Request(),
			presentKey: "native_protocol_version",
			absentKey:  "profile",
		},
		{
			name:       "Canary profile only",
			request:    validCanaryV2Request(),
			presentKey: "profile",
			absentKey:  "native_protocol_version",
		},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			payload, err := newV2Payload(test.request)
			if err != nil {
				t.Fatalf("newV2Payload returned error: %v", err)
			}
			encoded, err := json.Marshal(payload)
			if err != nil {
				t.Fatalf("marshal payload: %v", err)
			}
			var fields map[string]json.RawMessage
			if err := json.Unmarshal(encoded, &fields); err != nil {
				t.Fatalf("decode payload fields: %v", err)
			}
			if _, present := fields[test.presentKey]; !present {
				t.Fatalf("missing required family identity key %q in %s", test.presentKey, encoded)
			}
			if _, present := fields[test.absentKey]; present {
				t.Fatalf("forbidden family identity key %q present in %s", test.absentKey, encoded)
			}
		})
	}
}

func TestReadyForFailsClosedOnContradictoryIdentity(t *testing.T) {
	request := validV2Request()
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_ = json.NewEncoder(w).Encode(map[string]any{
			"contract_version":         2,
			"ready":                    true,
			"world_id":                 request.WorldID,
			"channel_id":               request.ChannelID,
			"world_policy_revision":    request.WorldPolicyRevision,
			"endpoint_id":              "different-endpoint",
			"audience":                 request.Audience,
			"family":                   request.GameplaySelection.Family,
			"native_protocol_version":  request.GameplaySelection.NativeProtocolVersion,
			"transport":                request.GameplaySelection.Transport,
			"schema_revision":          request.GameplaySelection.SchemaRevision,
			"schema_sha256":            request.GameplaySelection.SchemaSHA256,
			"capabilities":             request.GameplaySelection.Capabilities,
			"capability_digest_sha256": request.GameplaySelection.CapabilityDigestSHA256,
		})
	}))
	defer server.Close()

	client := NewClient(server.URL, "session-service-token", server.Client())
	if err := client.ReadyFor(context.Background(), request); !errors.Is(err, gateway.ErrUnavailable) {
		t.Fatalf("expected fail-closed readiness mismatch, got %v", err)
	}
}

func TestCreateFailsClosedOnDependencyErrors(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusServiceUnavailable)
	}))
	defer server.Close()

	client := NewClient(server.URL, "session-service-token", server.Client())
	_, err := client.Create(context.Background(), gateway.SessionRequest{ContractVersion: 1, CanaryAccountID: 1001, WorldID: 1, LoginAttemptID: "attempt"})
	if !errors.Is(err, gateway.ErrUnavailable) {
		t.Fatalf("expected ErrUnavailable, got %v", err)
	}
}

func TestCreateRejectsIncompleteV2BeforeNetwork(t *testing.T) {
	client := NewClient("https://session.example.test", "session-service-token", http.DefaultClient)
	_, err := client.Create(context.Background(), gateway.SessionRequest{ContractVersion: 2})
	if !errors.Is(err, gateway.ErrUnavailable) {
		t.Fatalf("expected invalid v2 request to fail locally, got %v", err)
	}
}

func TestCreateRejectsNegativeV2SecurityGenerationBeforeNetwork(t *testing.T) {
	request := validV2Request()
	request.SecurityGeneration = -1
	client := NewClient("https://session.example.test", "session-service-token", http.DefaultClient)
	_, err := client.Create(context.Background(), request)
	if !errors.Is(err, gateway.ErrUnavailable) {
		t.Fatalf("expected negative generation to fail locally, got %v", err)
	}
}

func TestCreateRejectsContradictoryV2FamilyIdentityBeforeNetwork(t *testing.T) {
	tests := []struct {
		name   string
		mutate func(*gateway.GameplaySelection)
	}{
		{name: "native profile alias", mutate: func(selection *gateway.GameplaySelection) { selection.Profile = "native.alias" }},
		{name: "Canary native version", mutate: func(selection *gateway.GameplaySelection) {
			selection.Family = "canary"
			selection.Profile = "canary.current"
			selection.NativeProtocolVersion = 1
		}},
	}

	client := NewClient("https://session.example.test", "session-service-token", http.DefaultClient)
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			request := validV2Request()
			test.mutate(request.GameplaySelection)
			_, err := client.Create(context.Background(), request)
			if !errors.Is(err, gateway.ErrUnavailable) {
				t.Fatalf("expected contradictory family identity to fail locally, got %v", err)
			}
		})
	}
}

func TestReadyChecksSessionServiceHealth(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/health" {
			t.Fatalf("unexpected readiness path: %s", r.URL.Path)
		}
		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	client := NewClient(server.URL, "session-service-token", server.Client())
	if err := client.Ready(context.Background()); err != nil {
		t.Fatalf("Ready returned error: %v", err)
	}
}

func validV2Request() gateway.SessionRequest {
	capabilities := []string{"actions.command-result.v1", "session.single-admission.v1"}
	return gateway.SessionRequest{
		ContractVersion:      2,
		CanaryAccountID:      1001,
		SecurityGeneration:   0,
		WorldID:              1,
		ChannelID:            1,
		LoginAttemptID:       strings.Repeat("a", 32),
		WorldPolicyRevision:  42,
		EndpointID:           "native-eu-1",
		Audience:             "otheryn-world:1:channel:1:endpoint:native-eu-1",
		CharacterBindingMode: "bind_on_first_admission",
		SingleAdmission:      true,
		GameplaySelection: &gateway.GameplaySelection{
			PolicyRevision:         42,
			Family:                 "oteryn",
			NativeProtocolVersion:  1,
			Transport:              "tcp.tls13.protobuf.be32.v1",
			SchemaRevision:         2,
			SchemaSHA256:           strings.Repeat("b", 64),
			Capabilities:           capabilities,
			CapabilityDigestSHA256: strings.Repeat("c", 64),
			EndpointID:             "native-eu-1",
			Host:                   "native.example.test",
			Port:                   7173,
			TLSServerName:          "native.example.test",
		},
	}
}

func validCanaryV2Request() gateway.SessionRequest {
	request := validV2Request()
	request.EndpointID = "canary-eu-1"
	request.Audience = "otheryn-world:1:channel:1:endpoint:canary-eu-1"
	request.GameplaySelection = &gateway.GameplaySelection{
		PolicyRevision:         42,
		Family:                 "canary",
		Profile:                "canary.current",
		Transport:              "canary.sequence.v1",
		SchemaRevision:         1,
		SchemaSHA256:           strings.Repeat("d", 64),
		Capabilities:           []string{"session.single-admission.v1"},
		CapabilityDigestSHA256: strings.Repeat("e", 64),
		EndpointID:             "canary-eu-1",
		Host:                   "canary.example.test",
		Port:                   7172,
		TLSServerName:          "canary.example.test",
	}
	return request
}

func assertServiceAuthentication(t *testing.T, r *http.Request) {
	t.Helper()
	if r.Header.Get("Authorization") != "Bearer session-service-token" {
		t.Fatalf("missing session service authentication")
	}
	if r.URL.RawQuery != "" {
		t.Fatalf("sensitive fields must not be in query: %s", r.URL.RawQuery)
	}
}

func assertV2Payload(t *testing.T, payload v2RequestPayload, request gateway.SessionRequest) {
	t.Helper()
	selection := request.GameplaySelection
	if payload.ContractVersion != 2 ||
		payload.GameAccountID != request.CanaryAccountID ||
		payload.IdentityGeneration != request.SecurityGeneration ||
		payload.WorldID != request.WorldID ||
		payload.ChannelID != request.ChannelID ||
		payload.LoginAttemptID != request.LoginAttemptID ||
		payload.WorldPolicyRevision != request.WorldPolicyRevision ||
		payload.EndpointID != request.EndpointID ||
		payload.Audience != request.Audience ||
		payload.CharacterBindingMode != request.CharacterBindingMode ||
		!payload.SingleAdmission ||
		payload.Family != selection.Family ||
		payload.Profile != selection.Profile ||
		payload.NativeProtocolVersion != selection.NativeProtocolVersion ||
		payload.Transport != selection.Transport ||
		payload.SchemaRevision != selection.SchemaRevision ||
		payload.SchemaSHA256 != selection.SchemaSHA256 ||
		!reflect.DeepEqual(payload.Capabilities, selection.Capabilities) ||
		payload.CapabilityDigestSHA256 != selection.CapabilityDigestSHA256 {
		t.Fatalf("unexpected v2 payload: %#v", payload)
	}
}
