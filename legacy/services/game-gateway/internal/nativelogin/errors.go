package nativelogin

import (
	"encoding/json"
	"errors"
)

// Public §11.2 codes. SECURITY_TERMINAL rows are only ever public as CodeAuthenticationRequired.
const (
	CodeRequestMalformed          = "NATIVE_LOGIN_REQUEST_MALFORMED"
	CodeUnsupportedVersion        = "NATIVE_LOGIN_UNSUPPORTED_VERSION"
	CodeOfferUnsupported          = "NATIVE_LOGIN_OFFER_UNSUPPORTED"
	CodeAuthenticationRequired    = "NATIVE_LOGIN_AUTHENTICATION_REQUIRED"
	CodeAttemptConflict           = "NATIVE_LOGIN_ATTEMPT_CONFLICT"
	CodeCharacterConflict         = "NATIVE_LOGIN_CHARACTER_CONFLICT"
	CodeRouteUnavailable          = "NATIVE_LOGIN_ROUTE_UNAVAILABLE"
	CodeRateLimited               = "NATIVE_LOGIN_RATE_LIMITED"
	CodeReconciliationRequired    = "ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED"
	CodeGrantExpired              = "NATIVE_LOGIN_GRANT_EXPIRED"
	CodeUnavailable               = "NATIVE_LOGIN_UNAVAILABLE"
	codeTicketRejected            = "NATIVE_LOGIN_TICKET_REJECTED"
	codeAccountSecurityDenied     = "NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED"
	publicClassTemporarily        = "TEMPORARILY_UNAVAILABLE"
	publicClassRetryLogin         = "RETRY_LOGIN"
	publicClassClientUpdate       = "CLIENT_UPDATE_REQUIRED"
	publicClassAuthentication     = "AUTHENTICATION_REQUIRED"
	publicClassSessionUnavailable = "SESSION_UNAVAILABLE"
)

type errorSpec struct {
	status      int
	publicClass string
}

// publicErrors is the §11.2 mapping of each public code to its HTTP status and public class.
var publicErrors = map[string]errorSpec{
	CodeRequestMalformed:       {400, publicClassRetryLogin},
	CodeUnsupportedVersion:     {400, publicClassClientUpdate},
	CodeOfferUnsupported:       {409, publicClassClientUpdate},
	CodeAuthenticationRequired: {401, publicClassAuthentication},
	CodeAttemptConflict:        {409, publicClassRetryLogin},
	CodeCharacterConflict:      {409, publicClassSessionUnavailable},
	CodeRouteUnavailable:       {503, publicClassTemporarily},
	CodeRateLimited:            {429, publicClassTemporarily},
	CodeReconciliationRequired: {503, publicClassTemporarily},
	CodeGrantExpired:           {409, publicClassRetryLogin},
	CodeUnavailable:            {503, publicClassTemporarily},
}

// Refusal is one §11 error outcome. RetryAfterSeconds is set only for CodeRateLimited.
type Refusal struct {
	Code              string
	RetryAfterSeconds int
}

func refuse(code string) *Refusal { return &Refusal{Code: code} }

// Status is the §11.2 HTTP status of the refusal.
func (r *Refusal) Status() int { return publicErrors[r.Code].status }

type errorDetail struct {
	Code        string `json:"code"`
	PublicClass string `json:"public_class"`
}

type errorBody struct {
	ProtocolVersion int         `json:"protocol_version"`
	Error           errorDetail `json:"error"`
	AttemptRef      *string     `json:"attempt_ref"`
}

// Body is the exact §11.1 error body.
func (r *Refusal) Body(attemptRef *string) any {
	return errorBody{
		ProtocolVersion: protocolVersion,
		Error:           errorDetail{Code: r.Code, PublicClass: publicErrors[r.Code].publicClass},
		AttemptRef:      attemptRef,
	}
}

// parseIssuerError validates an issuer §11.1 body: a known public code whose HTTP status and
// public class match §11.2, and an attempt_ref that is null or the forwarded one. An uncollapsed
// SECURITY_TERMINAL code is collapsed here, so it never reaches a client.
func parseIssuerError(status int, body []byte, request Request) (*Refusal, error) {
	if checkJSON(body, 2) != nil {
		return nil, errMalformed
	}
	members, err := exactObject(body, []string{"protocol_version", "error", "attempt_ref"}, "attempt_ref")
	if err != nil {
		return nil, err
	}
	var version int
	var attemptRef *string
	if json.Unmarshal(members["protocol_version"], &version) != nil ||
		json.Unmarshal(members["attempt_ref"], &attemptRef) != nil ||
		version != protocolVersion ||
		(attemptRef != nil && *attemptRef != request.AttemptRef) {
		return nil, errMalformed
	}
	detailMembers, err := exactObject(members["error"], []string{"code", "public_class"})
	if err != nil {
		return nil, err
	}
	var detail errorDetail
	if json.Unmarshal(detailMembers["code"], &detail.Code) != nil ||
		json.Unmarshal(detailMembers["public_class"], &detail.PublicClass) != nil {
		return nil, errMalformed
	}
	if detail.Code == codeTicketRejected || detail.Code == codeAccountSecurityDenied {
		detail.Code = CodeAuthenticationRequired
	}
	spec, known := publicErrors[detail.Code]
	if !known || spec.status != status || spec.publicClass != detail.PublicClass {
		return nil, errors.New("issuer error outside the §11.2 mapping")
	}
	return refuse(detail.Code), nil
}
