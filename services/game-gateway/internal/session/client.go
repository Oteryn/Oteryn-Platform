package session

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"reflect"
	"strings"

	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/gateway"
)

type Client struct {
	baseURL string
	token   string
	http    *http.Client
}

func NewClient(baseURL, token string, httpClient *http.Client) *Client {
	return &Client{
		baseURL: strings.TrimRight(baseURL, "/"),
		token:   token,
		http:    httpClient,
	}
}

func (c *Client) Create(ctx context.Context, request gateway.SessionRequest) (gateway.Session, error) {
	if request.ContractVersion == 0 || request.ContractVersion == 1 {
		return c.createV1(ctx, request)
	}
	if request.ContractVersion != 2 || request.GameplaySelection == nil {
		return gateway.Session{}, gateway.ErrUnavailable
	}
	return c.createV2(ctx, request)
}

func (c *Client) createV1(ctx context.Context, request gateway.SessionRequest) (gateway.Session, error) {
	payload := struct {
		ProtocolVersion int    `json:"protocol_version"`
		CanaryAccountID int64  `json:"canary_account_id"`
		WorldID         int64  `json:"world_id"`
		LoginAttemptID  string `json:"login_attempt_id"`
	}{
		ProtocolVersion: 1,
		CanaryAccountID: request.CanaryAccountID,
		WorldID:         request.WorldID,
		LoginAttemptID:  request.LoginAttemptID,
	}

	var result struct {
		ProtocolVersion int             `json:"protocol_version"`
		Session         gateway.Session `json:"session"`
	}
	status, err := c.doJSON(ctx, "/internal/v1/game-sessions", payload, &result)
	if err != nil || status != http.StatusOK || result.ProtocolVersion != 1 {
		return gateway.Session{}, gateway.ErrUnavailable
	}
	return result.Session, nil
}

func (c *Client) createV2(ctx context.Context, request gateway.SessionRequest) (gateway.Session, error) {
	payload, err := newV2Payload(request)
	if err != nil {
		return gateway.Session{}, err
	}

	var result struct {
		ContractVersion int             `json:"contract_version"`
		Session         gateway.Session `json:"session"`
	}
	status, err := c.doJSON(ctx, "/internal/v2/game-sessions", payload, &result)
	if err != nil || status != http.StatusOK || result.ContractVersion != 2 {
		return gateway.Session{}, gateway.ErrUnavailable
	}
	return result.Session, nil
}

func (c *Client) ReadyFor(ctx context.Context, request gateway.SessionRequest) error {
	payload, err := newV2Payload(request)
	if err != nil {
		return err
	}

	var result struct {
		ContractVersion        int      `json:"contract_version"`
		Ready                  bool     `json:"ready"`
		WorldID                int64    `json:"world_id"`
		ChannelID              uint64   `json:"channel_id"`
		WorldPolicyRevision    uint64   `json:"world_policy_revision"`
		EndpointID             string   `json:"endpoint_id"`
		Audience               string   `json:"audience"`
		Family                 string   `json:"family"`
		Profile                string   `json:"profile"`
		Transport              string   `json:"transport"`
		SchemaRevision         uint32   `json:"schema_revision"`
		SchemaSHA256           string   `json:"schema_sha256"`
		Capabilities           []string `json:"capabilities"`
		CapabilityDigestSHA256 string   `json:"capability_digest_sha256"`
	}
	status, err := c.doJSON(ctx, "/internal/v2/game-sessions/readiness", payload, &result)
	if err != nil || status != http.StatusOK || !result.Ready || result.ContractVersion != 2 {
		return gateway.ErrUnavailable
	}

	selection := request.GameplaySelection
	if result.WorldID != request.WorldID ||
		result.ChannelID != request.ChannelID ||
		result.WorldPolicyRevision != request.WorldPolicyRevision ||
		result.EndpointID != request.EndpointID ||
		result.Audience != request.Audience ||
		result.Family != selection.Family ||
		result.Profile != selection.Profile ||
		result.Transport != selection.Transport ||
		result.SchemaRevision != selection.SchemaRevision ||
		result.SchemaSHA256 != selection.SchemaSHA256 ||
		!reflect.DeepEqual(result.Capabilities, selection.Capabilities) ||
		result.CapabilityDigestSHA256 != selection.CapabilityDigestSHA256 {
		return gateway.ErrUnavailable
	}

	return nil
}

func (c *Client) Ready(ctx context.Context) error {
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+"/health", nil)
	if err != nil {
		return fmt.Errorf("session readiness request: %w", gateway.ErrUnavailable)
	}

	response, err := c.http.Do(request)
	if err != nil {
		return fmt.Errorf("session readiness: %w", gateway.ErrUnavailable)
	}
	defer response.Body.Close()
	_, _ = io.Copy(io.Discard, io.LimitReader(response.Body, 4096))

	if response.StatusCode != http.StatusOK {
		return gateway.ErrUnavailable
	}
	return nil
}

type v2RequestPayload struct {
	ContractVersion        int      `json:"contract_version"`
	GameAccountID          int64    `json:"game_account_id"`
	IdentityGeneration     int64    `json:"identity_security_generation"`
	WorldID                int64    `json:"world_id"`
	ChannelID              uint64   `json:"channel_id"`
	LoginAttemptID         string   `json:"login_attempt_id"`
	WorldPolicyRevision    uint64   `json:"world_policy_revision"`
	EndpointID             string   `json:"endpoint_id"`
	Audience               string   `json:"audience"`
	CharacterBindingMode   string   `json:"character_binding_mode"`
	SingleAdmission        bool     `json:"single_admission"`
	Family                 string   `json:"family"`
	Profile                string   `json:"profile"`
	Transport              string   `json:"transport"`
	SchemaRevision         uint32   `json:"schema_revision"`
	SchemaSHA256           string   `json:"schema_sha256"`
	Capabilities           []string `json:"capabilities"`
	CapabilityDigestSHA256 string   `json:"capability_digest_sha256"`
}

func newV2Payload(request gateway.SessionRequest) (v2RequestPayload, error) {
	selection := request.GameplaySelection
	if request.ContractVersion != 2 ||
		selection == nil ||
		request.CanaryAccountID < 1 ||
		request.SecurityGeneration < 0 ||
		request.WorldID < 1 ||
		request.ChannelID != 1 ||
		request.LoginAttemptID == "" ||
		request.WorldPolicyRevision < 1 ||
		request.EndpointID == "" ||
		request.Audience == "" ||
		request.CharacterBindingMode != "bind_on_first_admission" ||
		!request.SingleAdmission {
		return v2RequestPayload{}, gateway.ErrUnavailable
	}

	return v2RequestPayload{
		ContractVersion:        2,
		GameAccountID:          request.CanaryAccountID,
		IdentityGeneration:     request.SecurityGeneration,
		WorldID:                request.WorldID,
		ChannelID:              request.ChannelID,
		LoginAttemptID:         request.LoginAttemptID,
		WorldPolicyRevision:    request.WorldPolicyRevision,
		EndpointID:             request.EndpointID,
		Audience:               request.Audience,
		CharacterBindingMode:   request.CharacterBindingMode,
		SingleAdmission:        request.SingleAdmission,
		Family:                 selection.Family,
		Profile:                selection.Profile,
		Transport:              selection.Transport,
		SchemaRevision:         selection.SchemaRevision,
		SchemaSHA256:           selection.SchemaSHA256,
		Capabilities:           selection.Capabilities,
		CapabilityDigestSHA256: selection.CapabilityDigestSHA256,
	}, nil
}

func (c *Client) doJSON(ctx context.Context, path string, payload any, target any) (int, error) {
	encoded, err := json.Marshal(payload)
	if err != nil {
		return 0, gateway.ErrUnavailable
	}

	httpRequest, err := http.NewRequestWithContext(ctx, http.MethodPost, c.baseURL+path, bytes.NewReader(encoded))
	if err != nil {
		return 0, gateway.ErrUnavailable
	}
	httpRequest.Header.Set("Authorization", "Bearer "+c.token)
	httpRequest.Header.Set("Accept", "application/json")
	httpRequest.Header.Set("Content-Type", "application/json")

	response, err := c.http.Do(httpRequest)
	if err != nil {
		return 0, gateway.ErrUnavailable
	}
	defer response.Body.Close()

	limited := io.LimitReader(response.Body, 64*1024)
	if response.StatusCode != http.StatusOK {
		_, _ = io.Copy(io.Discard, limited)
		return response.StatusCode, nil
	}

	decoder := json.NewDecoder(limited)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(target); err != nil {
		return response.StatusCode, gateway.ErrUnavailable
	}
	return response.StatusCode, nil
}
