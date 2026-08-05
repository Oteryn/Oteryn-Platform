package platform

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/gateway"
)

func TestRedeemUsesPrivateServiceAuthAndKeepsTicketOutOfURL(t *testing.T) {
	const ticket = "ticket-secret-never-in-url"
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost || r.URL.Path != "/internal/v1/game-auth/tickets/redeem" || r.URL.RawQuery != "" {
			t.Fatalf("unexpected request: %s %s", r.Method, r.URL.String())
		}
		if r.Header.Get("Authorization") != "Bearer platform-service-token" {
			t.Fatalf("missing service authentication")
		}
		if strings.Contains(r.URL.String(), ticket) {
			t.Fatalf("ticket leaked to request URL")
		}
		var payload struct {
			ProtocolVersion int    `json:"protocol_version"`
			Ticket          string `json:"ticket"`
			Audience        string `json:"audience"`
		}
		if err := json.NewDecoder(r.Body).Decode(&payload); err != nil {
			t.Fatalf("decode request: %v", err)
		}
		if payload.ProtocolVersion != 1 || payload.Ticket != ticket || payload.Audience != "oteryn-game-gateway" {
			t.Fatalf("unexpected redeem payload: %#v", payload)
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"protocol_version":1,"authorization":{"canary_account_id":1001,"security_generation":0,"redeemed_at":"2026-07-22T08:00:00Z"}}`))
	}))
	defer server.Close()

	client := NewClient(server.URL, "platform-service-token", server.Client())
	authorization, err := client.Redeem(context.Background(), ticket)
	if err != nil {
		t.Fatalf("Redeem returned error: %v", err)
	}
	if authorization.CanaryAccountID != 1001 || authorization.SecurityGeneration != 0 {
		t.Fatalf("unexpected authorization: %#v", authorization)
	}
}

func TestRedeemRejectsNegativeSecurityGeneration(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write([]byte(`{"protocol_version":1,"authorization":{"canary_account_id":1001,"security_generation":-1,"redeemed_at":"2026-07-22T08:00:00Z"}}`))
	}))
	defer server.Close()

	client := NewClient(server.URL, "platform-service-token", server.Client())
	_, err := client.Redeem(context.Background(), "ticket")
	if !errors.Is(err, gateway.ErrUnavailable) {
		t.Fatalf("expected negative generation to fail closed, got %v", err)
	}
}

func TestRedeemMapsUnauthorizedAndOutage(t *testing.T) {
	for _, test := range []struct {
		name   string
		status int
		want   error
	}{
		{name: "unauthorized", status: http.StatusUnauthorized, want: gateway.ErrInvalidLogin},
		{name: "outage", status: http.StatusServiceUnavailable, want: gateway.ErrUnavailable},
	} {
		t.Run(test.name, func(t *testing.T) {
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
				w.WriteHeader(test.status)
			}))
			defer server.Close()

			client := NewClient(server.URL, "service-token", server.Client())
			_, err := client.Redeem(context.Background(), "ticket")
			if !errors.Is(err, test.want) {
				t.Fatalf("expected %v, got %v", test.want, err)
			}
		})
	}
}

func TestLoginContextUsesExactAccountPathAndParsesBoundedProjection(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet || r.URL.Path != "/internal/v1/game-auth/accounts/1001/login-context" {
			t.Fatalf("unexpected request: %s %s", r.Method, r.URL.Path)
		}
		if r.Header.Get("Authorization") != "Bearer platform-service-token" {
			t.Fatalf("missing service authentication")
		}
		_, _ = w.Write([]byte(`{
			"protocol_version":1,
			"worlds":[{"id":1,"slug":"oteryn","name":"Oteryn","region":"EU","host":"game.example.test","port":7172}],
			"characters":[{"id":10,"name":"Alpha","level":100,"vocation":4,"world_id":1}],
			"gameplay_policy":{
				"revision":42,
				"channel_id":1,
				"candidates":[{
					"family":"oteryn",
					"nativeProtocolVersion":1,
					"transport":"tcp.tls13.protobuf.be32.v1",
					"schema_revision":2,
					"schema_sha256":"` + strings.Repeat("a", 64) + `",
					"required_capabilities":["session.single-admission.v1"],
					"optional_capabilities":[],
					"endpoint_id":"native-eu-1",
					"host":"native.example.test",
					"port":7173,
					"tls_server_name":"native.example.test"
				}]
			}
		}`))
	}))
	defer server.Close()

	client := NewClient(server.URL, "platform-service-token", server.Client())
	contextResult, err := client.LoginContext(context.Background(), 1001)
	if err != nil {
		t.Fatalf("LoginContext returned error: %v", err)
	}
	if len(contextResult.Worlds) != 1 || len(contextResult.Characters) != 1 || contextResult.Characters[0].Name != "Alpha" {
		t.Fatalf("unexpected login context: %#v", contextResult)
	}
	if contextResult.GameplayPolicy.Revision != 42 || contextResult.GameplayPolicy.ChannelID != 1 || len(contextResult.GameplayPolicy.Candidates) != 1 {
		t.Fatalf("unexpected gameplay policy: %#v", contextResult.GameplayPolicy)
	}
	if contextResult.GameplayPolicy.Candidates[0].EndpointID != "native-eu-1" {
		t.Fatalf("unexpected candidate: %#v", contextResult.GameplayPolicy.Candidates[0])
	}
}

func TestLoginContextRejectsUnknownPrivateResponseFields(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write([]byte(`{"protocol_version":1,"worlds":[],"characters":[],"gameplay_policy":{"revision":1,"channel_id":1,"candidates":[]},"secret":"unexpected"}`))
	}))
	defer server.Close()

	client := NewClient(server.URL, "platform-service-token", server.Client())
	_, err := client.LoginContext(context.Background(), 1001)
	if !errors.Is(err, gateway.ErrUnavailable) {
		t.Fatalf("expected unknown private response field to fail closed, got %v", err)
	}
}

func TestReadyChecksPlatformHealth(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/health" {
			t.Fatalf("unexpected readiness path: %s", r.URL.Path)
		}
		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	client := NewClient(server.URL, "platform-service-token", server.Client())
	if err := client.Ready(context.Background()); err != nil {
		t.Fatalf("Ready returned error: %v", err)
	}
}
