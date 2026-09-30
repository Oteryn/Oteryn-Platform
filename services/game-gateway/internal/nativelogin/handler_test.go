package nativelogin

import (
	"bytes"
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strconv"
	"strings"
	"testing"
	"time"
)

type fakeAdmitter struct {
	calls   int
	success Success
	refusal *Refusal
}

func (f *fakeAdmitter) Admit(_ context.Context, _ Request) (Success, *Refusal) {
	f.calls++
	return f.success, f.refusal
}

type testClock struct{ now time.Time }

func (c *testClock) Now() time.Time { return c.now }

func serveNative(handler *Handler, body, remoteAddr string) *httptest.ResponseRecorder {
	request := httptest.NewRequest(http.MethodPost, "/v1/login", strings.NewReader(body))
	request.RemoteAddr = remoteAddr
	recorder := httptest.NewRecorder()
	handler.Serve(recorder, request, []byte(body))
	return recorder
}

func decodeError(t *testing.T, recorder *httptest.ResponseRecorder) (string, *string) {
	t.Helper()
	var body struct {
		ProtocolVersion int `json:"protocol_version"`
		Error           struct {
			Code        string `json:"code"`
			PublicClass string `json:"public_class"`
		} `json:"error"`
		AttemptRef *string `json:"attempt_ref"`
	}
	decoder := json.NewDecoder(recorder.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&body); err != nil || body.ProtocolVersion != 2 {
		t.Fatalf("invalid §11.1 body: %v", err)
	}
	if spec := publicErrors[body.Error.Code]; spec.status != recorder.Code || spec.publicClass != body.Error.PublicClass {
		t.Fatalf("error %s/%s does not match status %d", body.Error.Code, body.Error.PublicClass, recorder.Code)
	}
	return body.Error.Code, body.AttemptRef
}

func TestHandlerReturnsIssuerSuccessAndLogsNoSecret(t *testing.T) {
	issuer := &fakeAdmitter{success: Success{
		ProtocolVersion: 2, AttemptRef: testAttemptRef, WorldID: testWorld, ChannelID: testChannel,
		Endpoint: Endpoint{Host: "game.example.test", Port: 7172, TLSServerName: "game.example.test", ALPN: "oteryn-game/1", ProtocolMajor: 1, TransportProfile: 1},
		Grant:    Grant{Profile: grantProfile, Token: testGrantToken, ValidForSeconds: 20},
	}}
	var logs bytes.Buffer
	handler := NewHandler(issuer, slog.New(slog.NewJSONHandler(&logs, nil)))

	recorder := serveNative(handler, validRequestJSON(), "192.0.2.1:5000")
	if recorder.Code != http.StatusOK || issuer.calls != 1 {
		t.Fatalf("expected 200 after one issuer call, got %d calls=%d", recorder.Code, issuer.calls)
	}
	want := successJSON(testChannel)
	want = strings.Replace(want, "game-eu1.example.test", "game.example.test", 2)
	if strings.TrimSpace(recorder.Body.String()) != want {
		t.Fatalf("response is not the exact §3.2 body:\n%s", recorder.Body.String())
	}
	if strings.Contains(logs.String(), testTicket) || strings.Contains(logs.String(), testGrantToken) || !strings.Contains(logs.String(), testAttemptRef) {
		t.Fatalf("unexpected native log content: %s", logs.String())
	}
}

func TestHandlerRefusesMalformedRequestsWithoutCallingTheIssuer(t *testing.T) {
	issuer := &fakeAdmitter{}
	handler := NewHandler(issuer, slog.New(slog.NewJSONHandler(&bytes.Buffer{}, nil)))

	recorder := serveNative(handler, strings.Replace(validRequestJSON(), `"channel_id":null`, `"channel_id":null,"x":1`, 1), "192.0.2.1:5000")
	code, echo := decodeError(t, recorder)
	if code != CodeRequestMalformed || echo == nil || *echo != testAttemptRef || issuer.calls != 0 {
		t.Fatalf("expected malformed with echo and no issuer call, got %s %v calls=%d", code, echo, issuer.calls)
	}

	recorder = serveNative(handler, strings.Replace(validRequestJSON(), testAttemptRef, "not-canonical", 1), "192.0.2.1:5000")
	if code, echo := decodeError(t, recorder); code != CodeRequestMalformed || echo != nil {
		t.Fatalf("a non-canonical attempt_ref is never echoed: %s %v", code, echo)
	}
}

func TestHandlerMapsIssuerRefusals(t *testing.T) {
	issuer := &fakeAdmitter{refusal: &Refusal{Code: CodeRateLimited, RetryAfterSeconds: 9}}
	handler := NewHandler(issuer, slog.New(slog.NewJSONHandler(&bytes.Buffer{}, nil)))

	recorder := serveNative(handler, validRequestJSON(), "192.0.2.1:5000")
	if code, _ := decodeError(t, recorder); code != CodeRateLimited || recorder.Header().Get("Retry-After") != "9" {
		t.Fatalf("expected issuer rate limit with Retry-After, got %s %q", code, recorder.Header().Get("Retry-After"))
	}
}

func TestHandlerLimitsSourceAddressPerMinute(t *testing.T) {
	clock := &testClock{now: time.Unix(1_800_000_000, 0)}
	issuer := &fakeAdmitter{refusal: refuse(CodeRouteUnavailable)}
	handler := newHandler(issuer, slog.New(slog.NewJSONHandler(&bytes.Buffer{}, nil)), clock.Now)

	for i := 0; i < sourceLimit; i++ {
		body := strings.Replace(validRequestJSON(), testTicket, "ticket-"+strconv.Itoa(i), 1)
		if recorder := serveNative(handler, body, "192.0.2.1:"+strconv.Itoa(5000+i)); recorder.Code != http.StatusServiceUnavailable {
			t.Fatalf("request %d should reach the issuer, got %d", i, recorder.Code)
		}
	}
	recorder := serveNative(handler, validRequestJSON(), "192.0.2.1:6000")
	if code, _ := decodeError(t, recorder); code != CodeRateLimited || recorder.Header().Get("Retry-After") != "60" || issuer.calls != sourceLimit {
		t.Fatalf("expected source limit, got %s retry=%q calls=%d", code, recorder.Header().Get("Retry-After"), issuer.calls)
	}
	if recorder := serveNative(handler, validRequestJSON(), "198.51.100.7:5000"); recorder.Code == http.StatusTooManyRequests {
		t.Fatalf("another source address must not share the limit")
	}

	clock.now = clock.now.Add(sourceWindow)
	if recorder := serveNative(handler, validRequestJSON(), "192.0.2.1:6000"); recorder.Code == http.StatusTooManyRequests {
		t.Fatalf("the source limit must reset after its window")
	}
}

func TestHandlerLimitsOneAttemptToSixRequestsInTotal(t *testing.T) {
	clock := &testClock{now: time.Unix(1_800_000_000, 0)}
	issuer := &fakeAdmitter{refusal: refuse(CodeReconciliationRequired)}
	handler := newHandler(issuer, slog.New(slog.NewJSONHandler(&bytes.Buffer{}, nil)), clock.Now)

	for i := 0; i < attemptLimit; i++ {
		serveNative(handler, validRequestJSON(), "192.0.2."+strconv.Itoa(i+1)+":5000")
		clock.now = clock.now.Add(15 * time.Second)
	}
	recorder := serveNative(handler, validRequestJSON(), "192.0.2.99:5000")
	if code, echo := decodeError(t, recorder); code != CodeRateLimited || echo == nil || issuer.calls != attemptLimit {
		t.Fatalf("expected the per-attempt limit, got %s calls=%d", code, issuer.calls)
	}

	otherTicket := strings.Replace(validRequestJSON(), testTicket, "another-ticket", 1)
	if recorder := serveNative(handler, otherTicket, "192.0.2.99:5000"); recorder.Code == http.StatusTooManyRequests {
		t.Fatalf("the same attempt_ref with another ticket is a separate key (the issuer answers the conflict)")
	}

	clock.now = clock.now.Add(attemptRetention)
	if recorder := serveNative(handler, validRequestJSON(), "192.0.2.99:5000"); recorder.Code == http.StatusTooManyRequests {
		t.Fatalf("the per-attempt count is dropped after its retention")
	}
}

func TestWindowLimiterFailsClosedWhenFull(t *testing.T) {
	clock := &testClock{now: time.Unix(1_800_000_000, 0)}
	limiter := newWindowLimiter(1, time.Minute, 2, clock.Now)
	if ok, _ := limiter.allow("a"); !ok {
		t.Fatal("first key refused")
	}
	if ok, _ := limiter.allow("b"); !ok {
		t.Fatal("second key refused")
	}
	if ok, retry := limiter.allow("c"); ok || retry != 60 {
		t.Fatalf("a full table must refuse new keys, got ok=%v retry=%d", ok, retry)
	}
	clock.now = clock.now.Add(time.Minute)
	if ok, _ := limiter.allow("c"); !ok {
		t.Fatal("expired windows must be swept to make room")
	}
	if ok, _ := limiter.allow("a"); !ok {
		t.Fatal("an expired key restarts its window")
	}
}
