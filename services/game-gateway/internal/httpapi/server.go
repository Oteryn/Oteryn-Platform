package httpapi

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"regexp"
	"time"

	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/nativelogin"
)

const loginRequestLimit = 16 * 1024

var safeRequestID = regexp.MustCompile(`^[A-Za-z0-9._-]{1,64}$`)

type Server struct {
	checker Readiness
	native  NativeLoginHandler
	version string
	logger  *slog.Logger
	mux     *http.ServeMux
}

// Readiness reports whether the gateway's dependencies are reachable.
type Readiness interface {
	Ready(ctx context.Context) error
}

// NativeLoginHandler serves the native branch (protocol_version 2) of POST /v1/login.
type NativeLoginHandler interface {
	Serve(w http.ResponseWriter, r *http.Request, body []byte)
}

func NewServer(ready Readiness, version string, logger *slog.Logger) *Server {
	server := &Server{
		checker: ready,
		version: version,
		logger:  logger,
		mux:     http.NewServeMux(),
	}
	server.mux.HandleFunc("GET /health", server.health)
	server.mux.HandleFunc("GET /ready", server.ready)
	server.mux.HandleFunc("GET /version", server.versionInfo)
	server.mux.HandleFunc("POST /v1/login", server.login)
	return server
}

// EnableNativeLogin routes protocol_version 2 login requests to the native branch. Without it
// (the default) every login request is answered with invalid_request.
func (s *Server) EnableNativeLogin(native NativeLoginHandler) {
	s.native = native
}

func (s *Server) Handler() http.Handler {
	return s.loggingMiddleware(s.mux)
}

func (s *Server) health(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, map[string]any{"status": "ok"})
}

func (s *Server) ready(w http.ResponseWriter, r *http.Request) {
	if err := s.checker.Ready(r.Context()); err != nil {
		writeJSON(w, http.StatusServiceUnavailable, map[string]any{"status": "not_ready"})
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"status": "ready"})
}

func (s *Server) versionInfo(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, map[string]any{
		"service": "oteryn-game-gateway",
		"version": s.version,
	})
}

func (s *Server) login(w http.ResponseWriter, r *http.Request) {
	setSensitiveResponseNoCache(w)

	if r.URL.RawQuery != "" {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "invalid_request"})
		return
	}

	body, err := io.ReadAll(http.MaxBytesReader(w, r.Body, loginRequestLimit))
	if err == nil && s.native != nil && nativelogin.IsNativeRequest(body) {
		s.native.Serve(w, r, body)
		return
	}

	// Native admission (protocol_version 2) is the only login protocol. Every other body,
	// including the retired protocol_version 1, is refused without reaching a dependency.
	writeJSON(w, http.StatusBadRequest, map[string]any{"error": "invalid_request"})
}

func setSensitiveResponseNoCache(w http.ResponseWriter) {
	w.Header().Set("Cache-Control", "no-store, no-cache, must-revalidate, private")
	w.Header().Set("Pragma", "no-cache")
	w.Header().Set("Expires", "0")
}

func writeJSON(w http.ResponseWriter, status int, value any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(value)
}

func (s *Server) loggingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		started := time.Now()
		requestID := requestID(r.Header.Get("X-Request-ID"))
		w.Header().Set("X-Request-ID", requestID)
		recorder := &statusRecorder{ResponseWriter: w, status: http.StatusOK}

		next.ServeHTTP(recorder, r)

		s.logger.Info("http_request",
			"request_id", requestID,
			"method", r.Method,
			"path", r.URL.Path,
			"status", recorder.status,
			"duration_ms", time.Since(started).Milliseconds(),
		)
	})
}

type statusRecorder struct {
	http.ResponseWriter
	status      int
	wroteHeader bool
}

func (r *statusRecorder) WriteHeader(status int) {
	if r.wroteHeader {
		return
	}
	r.wroteHeader = true
	r.status = status
	r.ResponseWriter.WriteHeader(status)
}

func requestID(candidate string) string {
	if safeRequestID.MatchString(candidate) {
		return candidate
	}

	buffer := make([]byte, 16)
	if _, err := rand.Read(buffer); err != nil {
		return "request-id-unavailable"
	}
	return hex.EncodeToString(buffer)
}
