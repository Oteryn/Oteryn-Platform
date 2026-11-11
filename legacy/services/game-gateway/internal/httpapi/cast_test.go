package httpapi

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestCastWatchIsDefaultOffAndHasNoCharacterLoginFallback(t *testing.T) {
	s := newTestServer(nil)
	response := httptest.NewRecorder()
	s.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, "/v1/casts/watch", strings.NewReader(`{"accountTicket":"account-only"}`)))
	if response.Code != http.StatusServiceUnavailable {
		t.Fatalf("default-off watch status %d", response.Code)
	}
	assertSensitiveResponseNoCache(t, response)
	handler := &recordingNativeHandler{}
	s.EnableCastWatch(handler)
	response = httptest.NewRecorder()
	s.Handler().ServeHTTP(response, httptest.NewRequest(http.MethodPost, "/v1/casts/watch", strings.NewReader(`{"accountTicket":"account-only"}`)))
	if response.Code != http.StatusTeapot || len(handler.bodies) != 1 {
		t.Fatal("cast did not use its separate account-only handler")
	}
}
