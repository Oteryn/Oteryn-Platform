package platform

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strconv"
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

func (c *Client) Redeem(ctx context.Context, ticket string) (gateway.Authorization, error) {
	payload := struct {
		ProtocolVersion int    `json:"protocol_version"`
		Ticket          string `json:"ticket"`
		Audience        string `json:"audience"`
	}{
		ProtocolVersion: 1,
		Ticket:          ticket,
		Audience:        "oteryn-game-gateway",
	}

	var response struct {
		Authorization struct {
			CanaryAccountID    int64 `json:"canary_account_id"`
			SecurityGeneration int64 `json:"security_generation"`
		} `json:"authorization"`
	}

	status, err := c.doJSON(ctx, http.MethodPost, "/internal/v1/game-auth/tickets/redeem", payload, &response)
	if err != nil {
		return gateway.Authorization{}, err
	}
	if status == http.StatusUnauthorized {
		return gateway.Authorization{}, gateway.ErrInvalidLogin
	}
	if status != http.StatusOK {
		return gateway.Authorization{}, gateway.ErrUnavailable
	}
	if response.Authorization.CanaryAccountID < 1 {
		return gateway.Authorization{}, gateway.ErrUnavailable
	}

	return gateway.Authorization{
		CanaryAccountID:    response.Authorization.CanaryAccountID,
		SecurityGeneration: response.Authorization.SecurityGeneration,
	}, nil
}

func (c *Client) LoginContext(ctx context.Context, canaryAccountID int64) (gateway.LoginContext, error) {
	if canaryAccountID < 1 {
		return gateway.LoginContext{}, gateway.ErrUnavailable
	}

	var response struct {
		ProtocolVersion int                    `json:"protocol_version"`
		Worlds          []gateway.World        `json:"worlds"`
		Characters      []gateway.Character    `json:"characters"`
		GameplayPolicy  gateway.GameplayPolicy `json:"gameplay_policy"`
	}

	path := "/internal/v1/game-auth/accounts/" + strconv.FormatInt(canaryAccountID, 10) + "/login-context"
	status, err := c.doJSON(ctx, http.MethodGet, path, nil, &response)
	if err != nil {
		return gateway.LoginContext{}, err
	}
	if status != http.StatusOK || response.ProtocolVersion != 1 {
		return gateway.LoginContext{}, gateway.ErrUnavailable
	}

	return gateway.LoginContext{
		Worlds:         response.Worlds,
		Characters:     response.Characters,
		GameplayPolicy: response.GameplayPolicy,
	}, nil
}

func (c *Client) Ready(ctx context.Context) error {
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+"/health", nil)
	if err != nil {
		return fmt.Errorf("platform readiness request: %w", gateway.ErrUnavailable)
	}

	response, err := c.http.Do(request)
	if err != nil {
		return fmt.Errorf("platform readiness: %w", gateway.ErrUnavailable)
	}
	defer response.Body.Close()
	_, _ = io.Copy(io.Discard, io.LimitReader(response.Body, 4096))

	if response.StatusCode != http.StatusOK {
		return gateway.ErrUnavailable
	}
	return nil
}

func (c *Client) doJSON(ctx context.Context, method, path string, payload any, target any) (int, error) {
	var body io.Reader
	if payload != nil {
		encoded, err := json.Marshal(payload)
		if err != nil {
			return 0, fmt.Errorf("encode platform request: %w", gateway.ErrUnavailable)
		}
		body = bytes.NewReader(encoded)
	}

	request, err := http.NewRequestWithContext(ctx, method, c.baseURL+path, body)
	if err != nil {
		return 0, fmt.Errorf("create platform request: %w", gateway.ErrUnavailable)
	}
	request.Header.Set("Authorization", "Bearer "+c.token)
	request.Header.Set("Accept", "application/json")
	if payload != nil {
		request.Header.Set("Content-Type", "application/json")
	}

	response, err := c.http.Do(request)
	if err != nil {
		return 0, fmt.Errorf("platform request: %w", gateway.ErrUnavailable)
	}
	defer response.Body.Close()

	limited := io.LimitReader(response.Body, 64*1024)
	if response.StatusCode >= 200 && response.StatusCode < 300 {
		decoder := json.NewDecoder(limited)
		decoder.DisallowUnknownFields()
		if err := decoder.Decode(target); err != nil {
			return response.StatusCode, fmt.Errorf("decode platform response: %w", gateway.ErrUnavailable)
		}
		return response.StatusCode, nil
	}

	_, _ = io.Copy(io.Discard, limited)
	if response.StatusCode == http.StatusUnauthorized {
		return response.StatusCode, nil
	}
	return response.StatusCode, nil
}

func IsInvalidLogin(err error) bool {
	return errors.Is(err, gateway.ErrInvalidLogin)
}
