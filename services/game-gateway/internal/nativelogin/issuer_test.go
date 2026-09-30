package nativelogin

import (
	"context"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

const (
	testServiceToken = "synthetic-gateway-service-token"
	testGrantToken   = "eyJhbGciOiJFZDI1NTE5In0.eyJqdGkiOiJ4In0.c2lnbmF0dXJl"
)

func successJSON(channel string) string {
	return `{"protocol_version":2,"attempt_ref":"` + testAttemptRef + `","world_id":"` + testWorld + `","channel_id":"` + channel +
		`","endpoint":{"host":"game-eu1.example.test","port":7172,"tls_server_name":"game-eu1.example.test","alpn":"oteryn-game/1",` +
		`"protocol_major":1,"transport_profile":1},"grant":{"profile":"oteryn-pre-admission-v1","token":"` + testGrantToken + `","valid_for_seconds":20}}`
}

func errorJSON(code, class string, attemptRef string) string {
	return `{"protocol_version":2,"error":{"code":"` + code + `","public_class":"` + class + `"},"attempt_ref":` + attemptRef + `}`
}

func mustRequest(t *testing.T, body string) Request {
	t.Helper()
	request, refusal := ParseRequest([]byte(body))
	if refusal != nil {
		t.Fatalf("unexpected refusal %s", refusal.Code)
	}
	return request
}

func issuerReplying(t *testing.T, status int, body string, headers map[string]string) (*httptest.Server, *atomic.Int32) {
	t.Helper()
	var calls atomic.Int32
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		calls.Add(1)
		if r.Method != http.MethodPost || r.URL.Path != issuerPath || r.URL.RawQuery != "" {
			t.Errorf("unexpected issuer call %s %s", r.Method, r.URL.String())
		}
		if r.Header.Get("Authorization") != "Bearer "+testServiceToken || r.Header.Get("Content-Type") != "application/json" {
			t.Errorf("issuer call without the Gateway service credential or JSON content type")
		}
		forwarded, _ := io.ReadAll(r.Body)
		if string(forwarded) != validRequestJSON() {
			t.Errorf("forwarded body differs from the canonical request: %s", forwarded)
		}
		for name, value := range headers {
			w.Header().Set(name, value)
		}
		w.WriteHeader(status)
		_, _ = io.WriteString(w, body)
	}))
	t.Cleanup(server.Close)
	return server, &calls
}

func TestIssuerClientReturnsValidatedSuccess(t *testing.T) {
	server, calls := issuerReplying(t, http.StatusOK, successJSON(testChannel), nil)
	client := NewIssuerClient(server.URL+"/", testServiceToken, time.Second, nil)

	success, refusal := client.Admit(context.Background(), mustRequest(t, validRequestJSON()))
	if refusal != nil {
		t.Fatalf("unexpected refusal %s", refusal.Code)
	}
	if calls.Load() != 1 || success.Grant.Token != testGrantToken || success.ChannelID != testChannel || success.Endpoint.Port != 7172 {
		t.Fatalf("unexpected success %#v calls=%d", success, calls.Load())
	}
	if strings.Contains(success.String(), testGrantToken) {
		t.Fatalf("success rendering leaks the grant")
	}
}

func TestIssuerClientTreatsAnInvalidSuccessBodyAsReconciliation(t *testing.T) {
	valid := successJSON(testChannel)
	cases := map[string]string{
		"other attempt_ref": strings.Replace(valid, testAttemptRef, "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a70", 1),
		"extra member":      strings.Replace(valid, `"protocol_version":2,`, `"protocol_version":2,"debug":1,`, 1),
		"duplicate member":  strings.Replace(valid, `"protocol_version":2,`, `"protocol_version":2,"protocol_version":2,`, 1),
		"version one":       strings.Replace(valid, `"protocol_version":2`, `"protocol_version":1`, 1),
		"world not uuidv7":  strings.Replace(valid, testWorld, "1", 1),
		"other profile":     strings.Replace(valid, "oteryn-pre-admission-v1", "oteryn-reauth-recovery-v1", 1),
		"token not jws":     strings.Replace(valid, testGrantToken, "not-a-jws", 1),
		"token too long":    strings.Replace(valid, testGrantToken, "a."+strings.Repeat("b", 4094)+".c", 1),
		"zero validity":     strings.Replace(valid, `"valid_for_seconds":20`, `"valid_for_seconds":0`, 1),
		"validity above 30": strings.Replace(valid, `"valid_for_seconds":20`, `"valid_for_seconds":31`, 1),
		"port zero":         strings.Replace(valid, `"port":7172`, `"port":0`, 1),
		"ip server name":    strings.Replace(valid, `"tls_server_name":"game-eu1.example.test"`, `"tls_server_name":"10.0.0.1"`, 1),
		"host with scheme":  strings.Replace(valid, `"host":"game-eu1.example.test"`, `"host":"https://game"`, 1),
		"other alpn":        strings.Replace(valid, `"alpn":"oteryn-game/1"`, `"alpn":"h2"`, 1),
		"null grant":        valid[:strings.Index(valid, `"grant":`)] + `"grant":null}`,
		"oversized":         strings.Replace(valid, `"protocol_version":2,`, `"protocol_version":2,`+strings.Repeat(" ", MaxIssuerResponseBytes), 1),
		"not json":          `<html>`,
		"trailing json":     valid + `{}`,
		"missing endpoint":  strings.Replace(valid, valid[strings.Index(valid, `"endpoint"`):strings.Index(valid, `"grant"`)], ``, 1),
	}
	for name, body := range cases {
		t.Run(name, func(t *testing.T) {
			server, _ := issuerReplying(t, http.StatusOK, body, nil)
			client := NewIssuerClient(server.URL, testServiceToken, time.Second, nil)
			if _, refusal := client.Admit(context.Background(), mustRequest(t, validRequestJSON())); refusal == nil || refusal.Code != CodeReconciliationRequired {
				t.Fatalf("expected %s, got %#v", CodeReconciliationRequired, refusal)
			}
		})
	}

	ipHost := strings.Replace(valid, `"host":"game-eu1.example.test"`, `"host":"192.0.2.10"`, 1)
	server, _ := issuerReplying(t, http.StatusOK, ipHost, nil)
	if _, refusal := NewIssuerClient(server.URL, testServiceToken, time.Second, nil).Admit(context.Background(), mustRequest(t, validRequestJSON())); refusal != nil {
		t.Fatalf("an IP literal endpoint host is allowed (§3.2), got %s", refusal.Code)
	}
}

func TestIssuerClientRequiresTheRequestedChannel(t *testing.T) {
	requested := strings.Replace(validRequestJSON(), `"channel_id":null`, `"channel_id":"`+testChannel+`"`, 1)
	other := "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a70"
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = io.WriteString(w, successJSON(other))
	}))
	defer server.Close()
	client := NewIssuerClient(server.URL, testServiceToken, time.Second, nil)
	if _, refusal := client.Admit(context.Background(), mustRequest(t, requested)); refusal == nil || refusal.Code != CodeReconciliationRequired {
		t.Fatalf("a grant for another channel than requested must never reach the client: %#v", refusal)
	}
}

func TestIssuerClientPassesThroughContractErrors(t *testing.T) {
	for code, spec := range publicErrors {
		t.Run(code, func(t *testing.T) {
			server, _ := issuerReplying(t, spec.status, errorJSON(code, spec.publicClass, `"`+testAttemptRef+`"`), map[string]string{"Retry-After": "17"})
			client := NewIssuerClient(server.URL, testServiceToken, time.Second, nil)
			_, refusal := client.Admit(context.Background(), mustRequest(t, validRequestJSON()))
			if refusal == nil || refusal.Code != code {
				t.Fatalf("expected %s, got %#v", code, refusal)
			}
			if code == CodeRateLimited && refusal.RetryAfterSeconds != 17 {
				t.Fatalf("expected Retry-After passthrough, got %d", refusal.RetryAfterSeconds)
			}
		})
	}
}

func TestIssuerClientCollapsesSecurityTerminalCodes(t *testing.T) {
	for _, code := range []string{codeTicketRejected, codeAccountSecurityDenied} {
		server, _ := issuerReplying(t, http.StatusUnauthorized, errorJSON(code, publicClassAuthentication, `null`), nil)
		_, refusal := NewIssuerClient(server.URL, testServiceToken, time.Second, nil).Admit(context.Background(), mustRequest(t, validRequestJSON()))
		if refusal == nil || refusal.Code != CodeAuthenticationRequired {
			t.Fatalf("%s must reach the client only as %s, got %#v", code, CodeAuthenticationRequired, refusal)
		}
	}
}

func TestIssuerClientFailsClosedOnUnexpectedErrorAnswers(t *testing.T) {
	cases := map[string]struct {
		status int
		body   string
	}{
		"credential refused by middleware": {http.StatusUnauthorized, `{"error":"unauthorized"}`},
		"status does not match code":       {http.StatusServiceUnavailable, errorJSON(CodeAttemptConflict, publicClassRetryLogin, `null`)},
		"class does not match code":        {http.StatusConflict, errorJSON(CodeAttemptConflict, publicClassTemporarily, `null`)},
		"unknown code":                     {http.StatusConflict, errorJSON("SOMETHING_ELSE", publicClassRetryLogin, `null`)},
		"echo of another attempt":          {http.StatusConflict, errorJSON(CodeAttemptConflict, publicClassRetryLogin, `"0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a70"`)},
		"server error page":                {http.StatusInternalServerError, `<html>`},
		"created instead of ok":            {http.StatusCreated, successJSON(testChannel)},
	}
	for name, tc := range cases {
		t.Run(name, func(t *testing.T) {
			server, _ := issuerReplying(t, tc.status, tc.body, nil)
			_, refusal := NewIssuerClient(server.URL, testServiceToken, time.Second, nil).Admit(context.Background(), mustRequest(t, validRequestJSON()))
			if refusal == nil || refusal.Code != CodeUnavailable {
				t.Fatalf("expected %s, got %#v", CodeUnavailable, refusal)
			}
		})
	}
}

func TestIssuerClientDefaultsMissingRetryAfter(t *testing.T) {
	server, _ := issuerReplying(t, http.StatusTooManyRequests, errorJSON(CodeRateLimited, publicClassTemporarily, `null`), nil)
	_, refusal := NewIssuerClient(server.URL, testServiceToken, time.Second, nil).Admit(context.Background(), mustRequest(t, validRequestJSON()))
	if refusal == nil || refusal.Code != CodeRateLimited || refusal.RetryAfterSeconds != defaultRetryAfter {
		t.Fatalf("expected default Retry-After, got %#v", refusal)
	}
}

func TestIssuerClientNeverFollowsRedirects(t *testing.T) {
	var targetCalls atomic.Int32
	target := httptest.NewServer(http.HandlerFunc(func(http.ResponseWriter, *http.Request) { targetCalls.Add(1) }))
	defer target.Close()
	redirecting := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, target.URL+issuerPath, http.StatusTemporaryRedirect)
	}))
	defer redirecting.Close()

	_, refusal := NewIssuerClient(redirecting.URL, testServiceToken, time.Second, nil).Admit(context.Background(), mustRequest(t, validRequestJSON()))
	if refusal == nil || refusal.Code != CodeUnavailable || targetCalls.Load() != 0 {
		t.Fatalf("redirect must fail closed without a second call: %#v target=%d", refusal, targetCalls.Load())
	}
}

func TestIssuerClientDistinguishesUnsentFromUnknownOutcome(t *testing.T) {
	closed := httptest.NewServer(http.NotFoundHandler())
	closedURL := closed.URL
	closed.Close()
	_, refusal := NewIssuerClient(closedURL, testServiceToken, time.Second, nil).Admit(context.Background(), mustRequest(t, validRequestJSON()))
	if refusal == nil || refusal.Code != CodeUnavailable {
		t.Fatalf("a request that never left the Gateway is %s, got %#v", CodeUnavailable, refusal)
	}

	release := make(chan struct{})
	slow := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = io.ReadAll(r.Body)
		<-release
	}))
	defer slow.Close()
	defer close(release)
	_, refusal = NewIssuerClient(slow.URL, testServiceToken, 100*time.Millisecond, nil).Admit(context.Background(), mustRequest(t, validRequestJSON()))
	if refusal == nil || refusal.Code != CodeReconciliationRequired {
		t.Fatalf("a timeout after the request was written is %s, got %#v", CodeReconciliationRequired, refusal)
	}
}
