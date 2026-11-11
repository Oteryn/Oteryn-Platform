package nativelogin

import (
	"context"
	"crypto/sha256"
	"encoding/json"
	"log/slog"
	"net"
	"net/http"
	"strconv"
	"time"
)

const (
	sourceLimit  = 30 // §10: per source IP per minute
	sourceWindow = time.Minute
	attemptLimit = 6 // §10: per (attempt_ref, ticket hash) in total
	// attemptRetention outlives any usable attempt: a ticket lives at most 60 s and a grant
	// committed before the ticket expired at most 30 s more. Later requests for the pair can only
	// be refused by the issuer, and remain bounded by the source limit.
	attemptRetention     = 2 * time.Minute
	limiterMaxKeys       = 100_000
	outcomeSuccess       = "OK"
	logEventNativeResult = "native_login_result"
)

// Admitter is the private native admission issuer.
type Admitter interface {
	Admit(ctx context.Context, request Request) (Success, *Refusal)
}

// Handler serves the native branch of POST /v1/login.
type Handler struct {
	issuer   Admitter
	logger   *slog.Logger
	sources  *windowLimiter
	attempts *windowLimiter
}

func NewHandler(issuer Admitter, logger *slog.Logger) *Handler {
	return newHandler(issuer, logger, time.Now)
}

func newHandler(issuer Admitter, logger *slog.Logger, now func() time.Time) *Handler {
	return &Handler{
		issuer:   issuer,
		logger:   logger,
		sources:  newWindowLimiter(sourceLimit, sourceWindow, limiterMaxKeys, now),
		attempts: newWindowLimiter(attemptLimit, attemptRetention, limiterMaxKeys, now),
	}
}

// Serve answers one native request whose body the caller has already read (at most the public
// read limit) and whose response cache headers it has already set. Rate limits apply before the
// issuer call and change no issuer state (§10).
func (h *Handler) Serve(w http.ResponseWriter, r *http.Request, body []byte) {
	echo := AttemptRefEcho(body)

	if allowed, retry := h.sources.allow(sourceKey(r.RemoteAddr)); !allowed {
		h.refuse(w, &Refusal{Code: CodeRateLimited, RetryAfterSeconds: retry}, echo)
		return
	}

	request, refusal := ParseRequest(body)
	if refusal != nil {
		h.refuse(w, refusal, echo)
		return
	}
	if allowed, retry := h.attempts.allow(attemptKey(request)); !allowed {
		h.refuse(w, &Refusal{Code: CodeRateLimited, RetryAfterSeconds: retry}, echo)
		return
	}

	success, refusal := h.issuer.Admit(r.Context(), request)
	if refusal != nil {
		h.refuse(w, refusal, echo)
		return
	}

	h.logger.Info(logEventNativeResult, "attempt_ref", request.AttemptRef, "code", outcomeSuccess)
	writeJSON(w, http.StatusOK, success)
}

func (h *Handler) refuse(w http.ResponseWriter, refusal *Refusal, echo *string) {
	attemptRef := ""
	if echo != nil {
		attemptRef = *echo
	}
	h.logger.Info(logEventNativeResult, "attempt_ref", attemptRef, "code", refusal.Code)
	if refusal.Code == CodeRateLimited {
		w.Header().Set("Retry-After", strconv.Itoa(max(1, refusal.RetryAfterSeconds)))
	}
	writeJSON(w, refusal.Status(), refusal.Body(echo))
}

// sourceKey is the connection's source address. The Gateway trusts no forwarding header.
func sourceKey(remoteAddr string) string {
	if host, _, err := net.SplitHostPort(remoteAddr); err == nil {
		return host
	}
	return remoteAddr
}

// attemptKey is one SHA-256 over the attempt_ref and the ticket, so the ticket itself is never kept.
func attemptKey(request Request) string {
	digest := sha256.Sum256([]byte(request.AttemptRef + "\x00" + request.GameLoginTicket))
	return string(digest[:])
}

func writeJSON(w http.ResponseWriter, status int, value any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	encoder := json.NewEncoder(w)
	encoder.SetEscapeHTML(false)
	_ = encoder.Encode(value)
}
