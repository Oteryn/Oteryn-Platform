// Package nativelogin is the Gateway side of the native login branch (protocol_version 2) of
// OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT: it validates and rate-limits the public §3.1 request,
// forwards it once to the private Platform native admission issuer (§3.3, Decision D1) and returns
// the validated §3.2 response or a §11.1 error. The Gateway holds no database or signing credential,
// no grant state and never assembles claims.
package nativelogin

import (
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"net"
	"regexp"
)

const (
	// MaxRequestBytes is the public and issuer request body limit (§3.1, §10).
	MaxRequestBytes = 2048
	// MaxIssuerResponseBytes is the issuer response body limit (§10).
	MaxIssuerResponseBytes = 6144

	protocolVersion = 2
	grantProfile    = "oteryn-pre-admission-v1"
	supportedALPN   = "oteryn-game/1"
	maxGrantSeconds = 30
	maxTokenBytes   = 4096
	maxJSONDepth    = 4 // request object, offer, transports, transport
	maxTransports   = 4
)

var (
	uuidV7       = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`)
	ticketValue  = regexp.MustCompile(`^[\x21-\x7e]{1,256}$`)
	clientBuild  = regexp.MustCompile(`^[\x21-\x7e]{1,64}$`)
	alpnValue    = regexp.MustCompile(`^[\x21-\x7e]{1,255}$`)
	dnsName      = regexp.MustCompile(`^(?i)[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*$`)
	numericLabel = regexp.MustCompile(`(?:^|\.)[0-9]+$`)
	jwsCompact   = regexp.MustCompile(`^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$`)
)

var clientPlatforms = map[string]struct{}{"windows": {}}

var errMalformed = errors.New("malformed native JSON")

// Transport is one exact §3.1 offer.transports entry.
type Transport struct {
	ProtocolMajor    int    `json:"protocol_major"`
	TransportProfile int    `json:"transport_profile"`
	ALPN             string `json:"alpn"`
}

// Offer is the exact §3.1 offer. It is diagnostics and transport negotiation only.
type Offer struct {
	ClientBuild    string      `json:"client_build"`
	ClientPlatform string      `json:"client_platform"`
	Transports     []Transport `json:"transports"`
}

// Request is a validated §3.1 request. It carries the bearer ticket, so it redacts itself in
// fmt and slog output; only the forwarded issuer body contains the ticket.
type Request struct {
	ProtocolVersion int     `json:"protocol_version"`
	GameLoginTicket string  `json:"game_login_ticket"`
	AttemptRef      string  `json:"attempt_ref"`
	CharacterID     string  `json:"character_id"`
	ChannelID       *string `json:"channel_id"`
	Offer           Offer   `json:"offer"`
}

func (r Request) String() string   { return "nativelogin.Request{attempt_ref:" + r.AttemptRef + "}" }
func (r Request) GoString() string { return r.String() }

// Endpoint is the §3.2 Registry route record returned to the client.
type Endpoint struct {
	Host             string `json:"host"`
	Port             int    `json:"port"`
	TLSServerName    string `json:"tls_server_name"`
	ALPN             string `json:"alpn"`
	ProtocolMajor    int    `json:"protocol_major"`
	TransportProfile int    `json:"transport_profile"`
}

// Grant is the §3.2 FND-04 fresh-entry grant.
type Grant struct {
	Profile         string `json:"profile"`
	Token           string `json:"token"`
	ValidForSeconds int    `json:"valid_for_seconds"`
}

// Success is the exact §3.2 success body, members in contract order.
type Success struct {
	ProtocolVersion int      `json:"protocol_version"`
	AttemptRef      string   `json:"attempt_ref"`
	WorldID         string   `json:"world_id"`
	ChannelID       string   `json:"channel_id"`
	Endpoint        Endpoint `json:"endpoint"`
	Grant           Grant    `json:"grant"`
}

func (s Success) String() string   { return "nativelogin.Success{attempt_ref:" + s.AttemptRef + "}" }
func (s Success) GoString() string { return s.String() }

// IsNativeRequest reports whether a JSON object body selects the native branch: its top-level
// protocol_version is exactly the integer 2. Every other body keeps the Canary-compatible path.
func IsNativeRequest(body []byte) bool {
	var members map[string]json.RawMessage
	if err := json.Unmarshal(body, &members); err != nil {
		return false
	}
	return string(members["protocol_version"]) == "2"
}

// AttemptRefEcho returns the body's attempt_ref when it parses as canonical, for the §11.1 echo.
func AttemptRefEcho(body []byte) *string {
	if len(body) == 0 || len(body) > MaxRequestBytes {
		return nil
	}
	var members map[string]json.RawMessage
	if err := json.Unmarshal(body, &members); err != nil {
		return nil
	}
	var attemptRef string
	if err := json.Unmarshal(members["attempt_ref"], &attemptRef); err != nil || !uuidV7.MatchString(attemptRef) {
		return nil
	}
	return &attemptRef
}

// ParseRequest validates the exact §3.1 request with the same rules as the issuer
// (NativeAdmissionWire, NativeAdmissionRequest): exact member sets without unknown, duplicate,
// missing or null members (channel_id alone is nullable), bounded nesting, canonical UUIDv7s,
// bounded ASCII strings and an offer with at least one supported transport.
func ParseRequest(body []byte) (Request, *Refusal) {
	malformed := refuse(CodeRequestMalformed)
	if len(body) == 0 || len(body) > MaxRequestBytes || checkJSON(body, maxJSONDepth) != nil {
		return Request{}, malformed
	}

	members, err := exactObject(body, []string{"protocol_version", "game_login_ticket", "attempt_ref", "character_id", "channel_id", "offer"}, "channel_id")
	if err != nil {
		return Request{}, malformed
	}
	var request Request
	if json.Unmarshal(members["protocol_version"], &request.ProtocolVersion) != nil {
		return Request{}, malformed
	}
	if request.ProtocolVersion != protocolVersion {
		return Request{}, refuse(CodeUnsupportedVersion)
	}
	if json.Unmarshal(members["game_login_ticket"], &request.GameLoginTicket) != nil ||
		json.Unmarshal(members["attempt_ref"], &request.AttemptRef) != nil ||
		json.Unmarshal(members["character_id"], &request.CharacterID) != nil ||
		json.Unmarshal(members["channel_id"], &request.ChannelID) != nil {
		return Request{}, malformed
	}
	if !ticketValue.MatchString(request.GameLoginTicket) ||
		!uuidV7.MatchString(request.AttemptRef) ||
		!uuidV7.MatchString(request.CharacterID) ||
		(request.ChannelID != nil && !uuidV7.MatchString(*request.ChannelID)) {
		return Request{}, malformed
	}

	offer, refusal := parseOffer(members["offer"])
	if refusal != nil {
		return Request{}, refusal
	}
	request.Offer = offer
	return request, nil
}

func parseOffer(raw json.RawMessage) (Offer, *Refusal) {
	malformed := refuse(CodeRequestMalformed)
	members, err := exactObject(raw, []string{"client_build", "client_platform", "transports"})
	if err != nil {
		return Offer{}, malformed
	}
	var offer Offer
	var transports []json.RawMessage
	if json.Unmarshal(members["client_build"], &offer.ClientBuild) != nil ||
		json.Unmarshal(members["client_platform"], &offer.ClientPlatform) != nil ||
		json.Unmarshal(members["transports"], &transports) != nil {
		return Offer{}, malformed
	}
	if _, known := clientPlatforms[offer.ClientPlatform]; !clientBuild.MatchString(offer.ClientBuild) || !known ||
		len(transports) < 1 || len(transports) > maxTransports {
		return Offer{}, malformed
	}

	supported := false
	for _, rawTransport := range transports {
		transportMembers, err := exactObject(rawTransport, []string{"protocol_major", "transport_profile", "alpn"})
		if err != nil {
			return Offer{}, malformed
		}
		var transport Transport
		if json.Unmarshal(transportMembers["protocol_major"], &transport.ProtocolMajor) != nil ||
			json.Unmarshal(transportMembers["transport_profile"], &transport.TransportProfile) != nil ||
			json.Unmarshal(transportMembers["alpn"], &transport.ALPN) != nil {
			return Offer{}, malformed
		}
		if !alpnValue.MatchString(transport.ALPN) ||
			transport.ProtocolMajor < 0 || transport.ProtocolMajor > 65535 ||
			transport.TransportProfile < 0 || transport.TransportProfile > 65535 {
			return Offer{}, malformed
		}
		supported = supported || isSupportedTransport(transport)
		offer.Transports = append(offer.Transports, transport)
	}
	if !supported {
		return Offer{}, refuse(CodeOfferUnsupported)
	}
	return offer, nil
}

func isSupportedTransport(transport Transport) bool {
	return transport.ProtocolMajor == 1 && transport.TransportProfile == 1 && transport.ALPN == supportedALPN
}

// EncodeIssuerRequest re-encodes the validated request compactly in contract member order. The
// issuer therefore parses exactly the values the Gateway validated, and the offer values it digests
// (§6.1) are stable across retries of the same request.
func EncodeIssuerRequest(request Request) ([]byte, error) {
	var buffer bytes.Buffer
	encoder := json.NewEncoder(&buffer)
	encoder.SetEscapeHTML(false)
	if err := encoder.Encode(request); err != nil {
		return nil, err
	}
	encoded := bytes.TrimSuffix(buffer.Bytes(), []byte("\n"))
	if len(encoded) > MaxRequestBytes {
		return nil, errMalformed
	}
	return encoded, nil
}

// parseSuccess validates the exact §3.2 issuer success body against the forwarded request.
func parseSuccess(body []byte, request Request) (Success, error) {
	if checkJSON(body, 3) != nil {
		return Success{}, errMalformed
	}
	members, err := exactObject(body, []string{"protocol_version", "attempt_ref", "world_id", "channel_id", "endpoint", "grant"})
	if err != nil {
		return Success{}, err
	}
	var success Success
	if json.Unmarshal(members["protocol_version"], &success.ProtocolVersion) != nil ||
		json.Unmarshal(members["attempt_ref"], &success.AttemptRef) != nil ||
		json.Unmarshal(members["world_id"], &success.WorldID) != nil ||
		json.Unmarshal(members["channel_id"], &success.ChannelID) != nil {
		return Success{}, errMalformed
	}
	endpoint, err := exactObject(members["endpoint"], []string{"host", "port", "tls_server_name", "alpn", "protocol_major", "transport_profile"})
	if err != nil ||
		json.Unmarshal(endpoint["host"], &success.Endpoint.Host) != nil ||
		json.Unmarshal(endpoint["port"], &success.Endpoint.Port) != nil ||
		json.Unmarshal(endpoint["tls_server_name"], &success.Endpoint.TLSServerName) != nil ||
		json.Unmarshal(endpoint["alpn"], &success.Endpoint.ALPN) != nil ||
		json.Unmarshal(endpoint["protocol_major"], &success.Endpoint.ProtocolMajor) != nil ||
		json.Unmarshal(endpoint["transport_profile"], &success.Endpoint.TransportProfile) != nil {
		return Success{}, errMalformed
	}
	grant, err := exactObject(members["grant"], []string{"profile", "token", "valid_for_seconds"})
	if err != nil ||
		json.Unmarshal(grant["profile"], &success.Grant.Profile) != nil ||
		json.Unmarshal(grant["token"], &success.Grant.Token) != nil ||
		json.Unmarshal(grant["valid_for_seconds"], &success.Grant.ValidForSeconds) != nil {
		return Success{}, errMalformed
	}

	if success.ProtocolVersion != protocolVersion ||
		success.AttemptRef != request.AttemptRef ||
		!uuidV7.MatchString(success.WorldID) ||
		!uuidV7.MatchString(success.ChannelID) ||
		(request.ChannelID != nil && success.ChannelID != *request.ChannelID) ||
		!validEndpoint(success.Endpoint) ||
		success.Grant.Profile != grantProfile ||
		len(success.Grant.Token) > maxTokenBytes ||
		!jwsCompact.MatchString(success.Grant.Token) ||
		success.Grant.ValidForSeconds < 1 || success.Grant.ValidForSeconds > maxGrantSeconds {
		return Success{}, errMalformed
	}
	return success, nil
}

func validEndpoint(endpoint Endpoint) bool {
	hostValid := net.ParseIP(endpoint.Host) != nil || validDNSName(endpoint.Host)
	return hostValid &&
		endpoint.Port >= 1 && endpoint.Port <= 65535 &&
		validDNSName(endpoint.TLSServerName) && !numericLabel.MatchString(endpoint.TLSServerName) &&
		isSupportedTransport(Transport{ProtocolMajor: endpoint.ProtocolMajor, TransportProfile: endpoint.TransportProfile, ALPN: endpoint.ALPN})
}

func validDNSName(name string) bool {
	return len(name) >= 1 && len(name) <= 253 && dnsName.MatchString(name)
}

// exactObject decodes one JSON object with exactly the given members, none null unless listed
// as nullable. Duplicate names are rejected earlier by checkJSON.
func exactObject(raw []byte, names []string, nullable ...string) (map[string]json.RawMessage, error) {
	var members map[string]json.RawMessage
	if err := json.Unmarshal(raw, &members); err != nil || members == nil || len(members) != len(names) {
		return nil, errMalformed
	}
	for _, name := range names {
		value, present := members[name]
		if !present {
			return nil, errMalformed
		}
		if string(value) == "null" && !contains(nullable, name) {
			return nil, errMalformed
		}
	}
	return members, nil
}

func contains(values []string, candidate string) bool {
	for _, value := range values {
		if value == candidate {
			return true
		}
	}
	return false
}

// checkJSON requires one complete JSON value with no duplicate object member names and at most
// maxDepth nested containers.
func checkJSON(body []byte, maxDepth int) error {
	decoder := json.NewDecoder(bytes.NewReader(body))
	if err := consumeValue(decoder, maxDepth); err != nil {
		return err
	}
	if _, err := decoder.Token(); !errors.Is(err, io.EOF) {
		return errMalformed
	}
	return nil
}

func consumeValue(decoder *json.Decoder, depthLeft int) error {
	token, err := decoder.Token()
	if err != nil {
		return err
	}
	delimiter, isDelimiter := token.(json.Delim)
	if !isDelimiter {
		return nil
	}
	if depthLeft < 1 {
		return errMalformed
	}
	switch delimiter {
	case '{':
		seen := map[string]struct{}{}
		for decoder.More() {
			keyToken, err := decoder.Token()
			if err != nil {
				return err
			}
			key, ok := keyToken.(string)
			if !ok {
				return errMalformed
			}
			if _, duplicate := seen[key]; duplicate {
				return errMalformed
			}
			seen[key] = struct{}{}
			if err := consumeValue(decoder, depthLeft-1); err != nil {
				return err
			}
		}
	case '[':
		for decoder.More() {
			if err := consumeValue(decoder, depthLeft-1); err != nil {
				return err
			}
		}
	default:
		return errMalformed
	}
	_, err = decoder.Token()
	return err
}
