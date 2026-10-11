package platform

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
)

// ErrUnavailable reports that the Platform dependency did not answer a readiness probe.
var ErrUnavailable = errors.New("platform unavailable")

// Client probes Platform readiness. Login traffic goes through the native admission issuer.
type Client struct {
	baseURL string
	http    *http.Client
}

func NewClient(baseURL string, httpClient *http.Client) *Client {
	return &Client{
		baseURL: strings.TrimRight(baseURL, "/"),
		http:    httpClient,
	}
}

func (c *Client) Ready(ctx context.Context) error {
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+"/health", nil)
	if err != nil {
		return fmt.Errorf("platform readiness request: %w", ErrUnavailable)
	}

	response, err := c.http.Do(request)
	if err != nil {
		return fmt.Errorf("platform readiness: %w", ErrUnavailable)
	}
	defer response.Body.Close()
	_, _ = io.Copy(io.Discard, io.LimitReader(response.Body, 4096))

	if response.StatusCode != http.StatusOK {
		return ErrUnavailable
	}
	return nil
}
