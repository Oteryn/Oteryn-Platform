package nativelogin

import (
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

type castTransport func(*http.Request) (*http.Response, error)

func (f castTransport) RoundTrip(r *http.Request) (*http.Response, error) { return f(r) }
func TestCastWatchForwardsOnlyAnAccountRequestAndValidatesTheObserverPurpose(t *testing.T) {
	calls := 0
	transport := castTransport(func(r *http.Request) (*http.Response, error) {
		calls++
		if r.URL.Path != "/internal/v1/casts/watch" || r.Header.Get("Authorization") != "Bearer task-test-token" {
			t.Fatal("wrong purpose route or credential")
		}
		return &http.Response{StatusCode: 200, Header: make(http.Header), Body: io.NopCloser(strings.NewReader(`{"purpose":"player-admission","ticket":"wrong"}`))}, nil
	})
	h := NewCastWatchHandler("https://platform.example.invalid", "task-test-token", time.Second, transport)
	body := []byte(`{"accountTicket":"account-only","attemptRef":"0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b","cast":"abcdef0123456789abcdef0123456789"}`)
	response := httptest.NewRecorder()
	h.Serve(response, httptest.NewRequest("POST", "/v1/casts/watch", nil), body)
	if response.Code != 409 || calls != 1 {
		t.Fatal("wrong-purpose success was not refused")
	}
	h.Serve(httptest.NewRecorder(), httptest.NewRequest("POST", "/v1/casts/watch", nil), append(body[:len(body)-1], []byte(`,"character_id":"injected"}`)...))
	if calls != 1 {
		t.Fatal("character request reached observer issuer")
	}
}
