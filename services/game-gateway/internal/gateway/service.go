package gateway

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"time"
)

type Service struct {
	platform PlatformClient
	sessions SessionIssuer
	now      func() time.Time
}

func NewService(platform PlatformClient, sessions SessionIssuer) *Service {
	return &Service{
		platform: platform,
		sessions: sessions,
		now:      time.Now,
	}
}

func (s *Service) Login(ctx context.Context, ticket string) (LoginResponse, error) {
	return s.LoginWithRequest(ctx, LoginRequest{
		ProtocolVersion: gatewayProtocolVersion,
		GameLoginTicket: ticket,
	})
}

func (s *Service) LoginWithRequest(ctx context.Context, request LoginRequest) (LoginResponse, error) {
	if err := ValidateLoginRequest(request); err != nil {
		return LoginResponse{}, err
	}

	authorization, err := s.platform.Redeem(ctx, request.GameLoginTicket)
	if err != nil {
		return LoginResponse{}, err
	}
	if authorization.CanaryAccountID < 1 {
		return LoginResponse{}, ErrUnavailable
	}

	loginContext, err := s.platform.LoginContext(ctx, authorization.CanaryAccountID)
	if err != nil {
		return LoginResponse{}, err
	}
	world, err := validateLoginContext(loginContext)
	if err != nil {
		return LoginResponse{}, err
	}

	loginAttemptID, err := randomID(16)
	if err != nil {
		return LoginResponse{}, fmt.Errorf("generate login attempt id: %w", ErrUnavailable)
	}

	if request.GameplayOffer == nil {
		return s.createLegacySession(ctx, authorization, loginContext, world, loginAttemptID)
	}

	selection, err := SelectGameplayCandidate(loginContext.GameplayPolicy, *request.GameplayOffer)
	if err != nil {
		return LoginResponse{}, err
	}
	v2Request, err := NewV2SessionRequest(authorization, world, loginAttemptID, selection)
	if err != nil {
		return LoginResponse{}, err
	}
	if err := s.sessions.ReadyFor(ctx, v2Request); err != nil {
		return LoginResponse{}, ErrUnavailable
	}

	session, err := s.sessions.Create(ctx, v2Request)
	if err != nil {
		return LoginResponse{}, err
	}
	if err := s.validateSession(session); err != nil {
		return LoginResponse{}, err
	}

	return LoginResponse{
		ProtocolVersion:            gatewayProtocolVersion,
		GameSessionContractVersion: gameSessionContractVersionV2,
		LoginAttemptID:             loginAttemptID,
		Session:                    session,
		GameplaySelection:          &selection,
		Worlds:                     loginContext.Worlds,
		Characters:                 loginContext.Characters,
	}, nil
}

func (s *Service) createLegacySession(
	ctx context.Context,
	authorization Authorization,
	loginContext LoginContext,
	world World,
	loginAttemptID string,
) (LoginResponse, error) {
	session, err := s.sessions.Create(ctx, SessionRequest{
		ContractVersion: 1,
		CanaryAccountID: authorization.CanaryAccountID,
		WorldID:         world.ID,
		LoginAttemptID:  loginAttemptID,
	})
	if err != nil {
		return LoginResponse{}, err
	}
	if err := s.validateSession(session); err != nil {
		return LoginResponse{}, err
	}

	return LoginResponse{
		ProtocolVersion: gatewayProtocolVersion,
		Session:         session,
		Worlds:          loginContext.Worlds,
		Characters:      loginContext.Characters,
	}, nil
}

func validateLoginContext(loginContext LoginContext) (World, error) {
	if len(loginContext.Worlds) != 1 {
		return World{}, ErrUnavailable
	}

	world := loginContext.Worlds[0]
	if world.ID < 1 || world.Host == "" || world.Port < 1 || world.Port > 65535 {
		return World{}, ErrUnavailable
	}
	for _, character := range loginContext.Characters {
		if character.ID < 1 || character.WorldID != world.ID || character.Name == "" {
			return World{}, ErrUnavailable
		}
	}

	return world, nil
}

func (s *Service) validateSession(session Session) error {
	if session.Credential == "" || !session.ExpiresAt.After(s.now()) {
		return ErrUnavailable
	}
	return nil
}

func (s *Service) Ready(ctx context.Context) error {
	if err := s.platform.Ready(ctx); err != nil {
		return err
	}
	if err := s.sessions.Ready(ctx); err != nil {
		return err
	}
	return nil
}

func randomID(bytes int) (string, error) {
	buffer := make([]byte, bytes)
	if _, err := rand.Read(buffer); err != nil {
		return "", err
	}
	return hex.EncodeToString(buffer), nil
}
