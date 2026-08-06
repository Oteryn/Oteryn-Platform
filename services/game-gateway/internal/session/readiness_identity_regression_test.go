package session

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/gateway"
)

func TestReadyForRejectsForbiddenIdentityKeysEvenWhenNull(t *testing.T) {
	for _, test := range []struct {
		name         string
		request      gateway.SessionRequest
		forbiddenKey string
	}{
		{name: "native profile null", request: validV2Request(), forbiddenKey: "profile"},
		{name: "Canary native version null", request: validCanaryV2Request(), forbiddenKey: "native_protocol_version"},
	} {
		t.Run(test.name, func(t *testing.T) {
			request := test.request
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
				response := validReadinessResponse(request)
				response[test.forbiddenKey] = nil
				_ = json.NewEncoder(w).Encode(response)
			}))
			defer server.Close()

			client := NewClient(server.URL, "session-service-token", server.Client())
			if err := client.ReadyFor(context.Background(), request); !errors.Is(err, gateway.ErrUnavailable) {
				t.Fatalf("expected forbidden readiness identity key to fail closed, got %v", err)
			}
		})
	}
}

func TestReadyForRejectsMissingRequiredIdentityKey(t *testing.T) {
	for _, test := range []struct {
		name        string
		request     gateway.SessionRequest
		requiredKey string
	}{
		{name: "native version absent", request: validV2Request(), requiredKey: "native_protocol_version"},
		{name: "Canary profile absent", request: validCanaryV2Request(), requiredKey: "profile"},
	} {
		t.Run(test.name, func(t *testing.T) {
			request := test.request
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
				response := validReadinessResponse(request)
				delete(response, test.requiredKey)
				_ = json.NewEncoder(w).Encode(response)
			}))
			defer server.Close()

			client := NewClient(server.URL, "session-service-token", server.Client())
			if err := client.ReadyFor(context.Background(), request); !errors.Is(err, gateway.ErrUnavailable) {
				t.Fatalf("expected missing readiness identity key to fail closed, got %v", err)
			}
		})
	}
}

func TestReadyForRejectsUnknownResponseFields(t *testing.T) {
	request := validV2Request()
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		response := validReadinessResponse(request)
		response["unexpected"] = true
		_ = json.NewEncoder(w).Encode(response)
	}))
	defer server.Close()

	client := NewClient(server.URL, "session-service-token", server.Client())
	if err := client.ReadyFor(context.Background(), request); !errors.Is(err, gateway.ErrUnavailable) {
		t.Fatalf("expected unknown readiness field to fail closed, got %v", err)
	}
}

func TestReadyForRejectsDuplicateAndTrailingJSON(t *testing.T) {
	request := validV2Request()
	canonical, err := json.Marshal(validReadinessResponse(request))
	if err != nil {
		t.Fatalf("marshal readiness response: %v", err)
	}

	for _, test := range []struct {
		name string
		body func([]byte) []byte
	}{
		{
			name: "duplicate identity key",
			body: func(payload []byte) []byte {
				result := append([]byte(nil), payload[:len(payload)-1]...)
				return append(result, []byte(`,"family":"oteryn"}`)...)
			},
		},
		{
			name: "trailing JSON document",
			body: func(payload []byte) []byte {
				return append(append([]byte(nil), payload...), []byte("\n{}")...)
			},
		},
	} {
		t.Run(test.name, func(t *testing.T) {
			body := test.body(canonical)
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
				w.Header().Set("Content-Type", "application/json")
				_, _ = w.Write(body)
			}))
			defer server.Close()

			client := NewClient(server.URL, "session-service-token", server.Client())
			if err := client.ReadyFor(context.Background(), request); !errors.Is(err, gateway.ErrUnavailable) {
				t.Fatalf("expected malformed readiness JSON to fail closed, got %v", err)
			}
		})
	}
}

func validReadinessResponse(request gateway.SessionRequest) map[string]any {
	selection := request.GameplaySelection
	response := map[string]any{
		"contract_version":         2,
		"ready":                    true,
		"world_id":                 request.WorldID,
		"channel_id":               request.ChannelID,
		"world_policy_revision":    request.WorldPolicyRevision,
		"endpoint_id":              request.EndpointID,
		"audience":                 request.Audience,
		"family":                   selection.Family,
		"transport":                selection.Transport,
		"schema_revision":          selection.SchemaRevision,
		"schema_sha256":            selection.SchemaSHA256,
		"capabilities":             selection.Capabilities,
		"capability_digest_sha256": selection.CapabilityDigestSHA256,
	}
	if selection.Family == "oteryn" {
		response["native_protocol_version"] = selection.NativeProtocolVersion
	} else {
		response["profile"] = selection.Profile
	}
	return response
}
