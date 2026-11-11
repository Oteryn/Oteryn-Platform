package nativelogin

import (
	"bytes"
	"context"
	"io"
	"net/http"
	"net/http/httptrace"
	"strconv"
	"strings"
	"sync/atomic"
	"time"
)

const (
	issuerPath          = "/internal/v1/game-auth/native-admissions"
	defaultRetryAfter   = 60
	maxRetryAfter       = 3600
	maxIssuerErrorBytes = 4096
)

// IssuerClient calls the private Platform native admission issuer with the Gateway service
// credential (§3.3). It never follows redirects, so the credential and ticket reach only the
// configured issuer.
type IssuerClient struct {
	url   string
	token string
	http  *http.Client
}

// NewIssuerClient builds the issuer client. timeout bounds one whole issuer exchange.
func NewIssuerClient(platformBaseURL, serviceToken string, timeout time.Duration, transport http.RoundTripper) *IssuerClient {
	return &IssuerClient{
		url:   strings.TrimRight(platformBaseURL, "/") + issuerPath,
		token: serviceToken,
		http: &http.Client{
			Timeout:   timeout,
			Transport: transport,
			CheckRedirect: func(*http.Request, []*http.Request) error {
				return http.ErrUseLastResponse
			},
		},
	}
}

// Admit forwards one validated request and returns the validated §3.2 success or a §11 refusal.
//
// Outcome mapping when the issuer gives no valid answer (§6.2 step 5): if the request never left
// the Gateway, nothing can have committed, so the result is NATIVE_LOGIN_UNAVAILABLE. Once it was
// written, the issuer may have committed a grant, so a transport failure, timeout, oversized or
// invalid success body is ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED: the client retries the same
// ticket and attempt_ref and the issuer returns the committed grant. An invalid error body, a
// redirect or an unexpected status is NATIVE_LOGIN_UNAVAILABLE (a refused request commits nothing).
func (c *IssuerClient) Admit(ctx context.Context, request Request) (Success, *Refusal) {
	body, err := EncodeIssuerRequest(request)
	if err != nil {
		return Success{}, refuse(CodeRequestMalformed)
	}

	var written atomic.Bool
	trace := &httptrace.ClientTrace{
		WroteRequest: func(httptrace.WroteRequestInfo) { written.Store(true) },
	}
	outbound, err := http.NewRequestWithContext(httptrace.WithClientTrace(ctx, trace), http.MethodPost, c.url, bytes.NewReader(body))
	if err != nil {
		return Success{}, refuse(CodeUnavailable)
	}
	outbound.Header.Set("Authorization", "Bearer "+c.token)
	outbound.Header.Set("Content-Type", "application/json")
	outbound.Header.Set("Accept", "application/json")

	unknownOutcome := func() (Success, *Refusal) {
		if written.Load() {
			return Success{}, refuse(CodeReconciliationRequired)
		}
		return Success{}, refuse(CodeUnavailable)
	}

	response, err := c.http.Do(outbound)
	if err != nil {
		return unknownOutcome()
	}
	defer response.Body.Close()

	if response.StatusCode == http.StatusOK {
		payload, err := io.ReadAll(io.LimitReader(response.Body, MaxIssuerResponseBytes+1))
		if err != nil || len(payload) > MaxIssuerResponseBytes {
			return unknownOutcome()
		}
		success, err := parseSuccess(payload, request)
		if err != nil {
			return unknownOutcome()
		}
		return success, nil
	}

	payload, err := io.ReadAll(io.LimitReader(response.Body, maxIssuerErrorBytes+1))
	if err != nil || len(payload) > maxIssuerErrorBytes {
		return Success{}, refuse(CodeUnavailable)
	}
	refusal, err := parseIssuerError(response.StatusCode, payload, request)
	if err != nil {
		return Success{}, refuse(CodeUnavailable)
	}
	if refusal.Code == CodeRateLimited {
		refusal.RetryAfterSeconds = retryAfter(response.Header.Get("Retry-After"))
	}
	return Success{}, refusal
}

func retryAfter(raw string) int {
	seconds, err := strconv.Atoi(raw)
	if err != nil || seconds < 1 || seconds > maxRetryAfter {
		return defaultRetryAfter
	}
	return seconds
}
