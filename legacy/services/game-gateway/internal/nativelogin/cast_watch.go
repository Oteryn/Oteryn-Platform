package nativelogin

import (
	"bytes"
	"encoding/json"
	"io"
	"net"
	"net/http"
	"regexp"
	"strings"
	"time"
)

var castUUID = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`)
var castID = regexp.MustCompile(`^[0-9a-f]{32}$`)
var castTicket = regexp.MustCompile(`^ocv1\.[0-9a-f]{64}$`)
var castHost = regexp.MustCompile(`^[A-Za-z0-9.:-]{1,253}$`)

type CastWatchHandler struct {
	url, token string
	http       *http.Client
	sources    *windowLimiter
}
type castWatchRequest struct {
	Chat          bool    `json:"chat"`
	AccountTicket string  `json:"accountTicket"`
	AttemptRef    string  `json:"attemptRef"`
	Cast          string  `json:"cast"`
	Password      *string `json:"password,omitempty"`
}
type castWatchSuccess struct {
	Chat      bool   `json:"chat"`
	Purpose   string `json:"purpose"`
	Ticket    string `json:"ticket"`
	Node      string `json:"node"`
	Host      string `json:"host"`
	Port      int    `json:"port"`
	ExpiresAt int64  `json:"expiresAt"`
}

func NewCastWatchHandler(base, token string, timeout time.Duration, transport http.RoundTripper) *CastWatchHandler {
	return &CastWatchHandler{url: strings.TrimRight(base, "/") + "/internal/v1/casts/watch", token: token,
		http:    &http.Client{Timeout: timeout, Transport: transport, CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }},
		sources: newWindowLimiter(20, time.Minute, 10_000, time.Now)}
}
func strictCastJSON(body []byte, destination any) bool {
	decoder := json.NewDecoder(bytes.NewReader(body))
	decoder.DisallowUnknownFields()
	if decoder.Decode(destination) != nil {
		return false
	}
	var extra any
	return decoder.Decode(&extra) == io.EOF
}

// Serve never logs a password, ticket, account, destination or issuer response.
// The Platform issuer alone authenticates the account and reserves global slots.
func (h *CastWatchHandler) Serve(w http.ResponseWriter, r *http.Request, body []byte) {
	refuse := func() { writeJSON(w, http.StatusConflict, map[string]string{"error": "Cast unavailable"}) }
	allowed, _ := h.sources.allow(sourceKey(r.RemoteAddr))
	var request castWatchRequest
	if !allowed || len(body) > 4096 || !strictCastJSON(body, &request) || len(request.AccountTicket) == 0 || len(request.AccountTicket) > 2048 ||
		!castUUID.MatchString(request.AttemptRef) || !castID.MatchString(request.Cast) ||
		(request.Password != nil && len(*request.Password) > 128) {
		refuse()
		return
	}
	source := sourceKey(r.RemoteAddr)
	if net.ParseIP(source) == nil {
		refuse()
		return
	}
	// Only the authenticated Gateway adds this field. Public JSON cannot supply it.
	issuerRequest := struct {
		castWatchRequest
		SourceIP string `json:"sourceIp"`
	}{request, source}
	canonical, err := json.Marshal(issuerRequest)
	if err != nil {
		refuse()
		return
	}
	outbound, err := http.NewRequestWithContext(r.Context(), http.MethodPost, h.url, bytes.NewReader(canonical))
	if err != nil {
		refuse()
		return
	}
	outbound.Header.Set("Authorization", "Bearer "+h.token)
	outbound.Header.Set("Content-Type", "application/json")
	response, err := h.http.Do(outbound)
	if err != nil {
		refuse()
		return
	}
	defer response.Body.Close()
	if response.StatusCode != 200 {
		refuse()
		return
	}
	result, err := io.ReadAll(io.LimitReader(response.Body, 1025))
	var success castWatchSuccess
	now := time.Now().Unix()
	if err != nil || len(result) > 1024 || !strictCastJSON(result, &success) || success.Purpose != "oteryn-cast-observer-v1" ||
		!castTicket.MatchString(success.Ticket) || !castUUID.MatchString(success.Node) || !castHost.MatchString(success.Host) ||
		success.Port < 1 || success.Port > 65535 || success.ExpiresAt <= now || success.ExpiresAt > now+5 {
		refuse()
		return
	}
	writeJSON(w, http.StatusOK, success)
}
