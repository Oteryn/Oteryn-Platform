package gateway

import (
	"context"
	"encoding/json"
	"errors"
	"time"
)

var (
	ErrInvalidRequest          = errors.New("invalid request")
	ErrInvalidLogin            = errors.New("invalid login")
	ErrUnsupportedGameplayPair = errors.New("unsupported gameplay pair")
	ErrUnavailable             = errors.New("login unavailable")
)

type Authorization struct {
	CanaryAccountID    int64
	SecurityGeneration int64
}

type World struct {
	ID     int64  `json:"id"`
	Slug   string `json:"slug"`
	Name   string `json:"name"`
	Region string `json:"region"`
	Host   string `json:"host"`
	Port   int    `json:"port"`
}

type Character struct {
	ID       int64  `json:"id"`
	Name     string `json:"name"`
	Level    int    `json:"level"`
	Vocation int    `json:"vocation"`
	WorldID  int64  `json:"world_id"`
}

type GameplayOfferCandidate struct {
	Family                string `json:"family"`
	Profile               string `json:"profile,omitempty"`
	NativeProtocolVersion uint32 `json:"native_protocol_version,omitempty"`
	profilePresent        bool
	nativeVersionPresent  bool
	Transport             string   `json:"transport"`
	SchemaRevision        uint32   `json:"schema_revision"`
	SchemaSHA256          string   `json:"schema_sha256"`
	Capabilities          []string `json:"capabilities"`
}

type GameplayOffer struct {
	OfferVersion   int                      `json:"offer_version"`
	ClientBuild    string                   `json:"client_build"`
	ClientPlatform string                   `json:"client_platform"`
	Candidates     []GameplayOfferCandidate `json:"candidates"`
}

type LoginRequest struct {
	ProtocolVersion int            `json:"protocol_version"`
	GameLoginTicket string         `json:"game_login_ticket"`
	GameplayOffer   *GameplayOffer `json:"gameplay_offer,omitempty"`
}

type GameplayPolicyCandidate struct {
	Family                string `json:"family"`
	Profile               string `json:"profile,omitempty"`
	NativeProtocolVersion uint32 `json:"native_protocol_version,omitempty"`
	profilePresent        bool
	nativeVersionPresent  bool
	Transport             string   `json:"transport"`
	SchemaRevision        uint32   `json:"schema_revision"`
	SchemaSHA256          string   `json:"schema_sha256"`
	RequiredCapabilities  []string `json:"required_capabilities"`
	OptionalCapabilities  []string `json:"optional_capabilities"`
	EndpointID            string   `json:"endpoint_id"`
	Host                  string   `json:"host"`
	Port                  int      `json:"port"`
	TLSServerName         string   `json:"tls_server_name"`
}

type GameplayPolicy struct {
	Revision   uint64                    `json:"revision"`
	ChannelID  uint64                    `json:"channel_id"`
	Candidates []GameplayPolicyCandidate `json:"candidates"`
}

type LoginContext struct {
	Worlds         []World
	Characters     []Character
	GameplayPolicy GameplayPolicy
}

type GameplaySelection struct {
	PolicyRevision         uint64   `json:"policy_revision"`
	Family                 string   `json:"family"`
	Profile                string   `json:"profile,omitempty"`
	NativeProtocolVersion  uint32   `json:"native_protocol_version,omitempty"`
	Transport              string   `json:"transport"`
	SchemaRevision         uint32   `json:"schema_revision"`
	SchemaSHA256           string   `json:"schema_sha256"`
	Capabilities           []string `json:"capabilities"`
	CapabilityDigestSHA256 string   `json:"capability_digest_sha256"`
	EndpointID             string   `json:"-"`
	Host                   string   `json:"host"`
	Port                   int      `json:"port"`
	TLSServerName          string   `json:"tls_server_name"`
}

type SessionRequest struct {
	ContractVersion      int
	CanaryAccountID      int64
	SecurityGeneration   int64
	WorldID              int64
	ChannelID            uint64
	LoginAttemptID       string
	WorldPolicyRevision  uint64
	EndpointID           string
	Audience             string
	CharacterBindingMode string
	SingleAdmission      bool
	GameplaySelection    *GameplaySelection
}

type Session struct {
	Credential string    `json:"credential"`
	ExpiresAt  time.Time `json:"expires_at"`
}

type LoginResponse struct {
	ProtocolVersion            int                `json:"protocol_version"`
	GameSessionContractVersion int                `json:"game_session_contract_version,omitempty"`
	LoginAttemptID             string             `json:"login_attempt_id,omitempty"`
	Session                    Session            `json:"session"`
	GameplaySelection          *GameplaySelection `json:"gameplay_selection,omitempty"`
	Worlds                     []World            `json:"worlds"`
	Characters                 []Character        `json:"characters"`
}

type PlatformClient interface {
	Redeem(ctx context.Context, ticket string) (Authorization, error)
	LoginContext(ctx context.Context, canaryAccountID int64) (LoginContext, error)
	Ready(ctx context.Context) error
}

type SessionIssuer interface {
	Create(ctx context.Context, request SessionRequest) (Session, error)
	Ready(ctx context.Context) error
	ReadyFor(ctx context.Context, request SessionRequest) error
}

func (candidate *GameplayOfferCandidate) UnmarshalJSON(data []byte) error {
	type alias GameplayOfferCandidate
	var decoded struct {
		alias
		Profile               *string `json:"profile"`
		NativeProtocolVersion *uint32 `json:"native_protocol_version"`
	}
	if err := json.Unmarshal(data, &decoded); err != nil {
		return err
	}
	*candidate = GameplayOfferCandidate(decoded.alias)
	if decoded.Profile != nil {
		candidate.Profile = *decoded.Profile
		candidate.profilePresent = true
	}
	if decoded.NativeProtocolVersion != nil {
		candidate.NativeProtocolVersion = *decoded.NativeProtocolVersion
		candidate.nativeVersionPresent = true
	}
	return nil
}

func (candidate *GameplayPolicyCandidate) UnmarshalJSON(data []byte) error {
	type alias GameplayPolicyCandidate
	var decoded struct {
		alias
		Profile               *string `json:"profile"`
		NativeProtocolVersion *uint32 `json:"native_protocol_version"`
	}
	if err := json.Unmarshal(data, &decoded); err != nil {
		return err
	}
	*candidate = GameplayPolicyCandidate(decoded.alias)
	if decoded.Profile != nil {
		candidate.Profile = *decoded.Profile
		candidate.profilePresent = true
	}
	if decoded.NativeProtocolVersion != nil {
		candidate.NativeProtocolVersion = *decoded.NativeProtocolVersion
		candidate.nativeVersionPresent = true
	}
	return nil
}
