package gateway

import (
	"context"
	"errors"
	"testing"
)

func TestLoginRejectsMoreThanOneHundredCharactersBeforeSessionIssuance(t *testing.T) {
	platform := successfulPlatform()
	platform.loginContext.Characters = make([]Character, 101)
	for index := range platform.loginContext.Characters {
		platform.loginContext.Characters[index] = Character{
			ID:      int64(index + 1),
			Name:    "Character",
			WorldID: 1,
		}
	}
	sessions := &fakeSessionIssuer{}
	service := NewService(platform, sessions)

	_, err := service.Login(context.Background(), "ticket")
	if !errors.Is(err, ErrUnavailable) {
		t.Fatalf("expected oversized character projection to fail closed, got %v", err)
	}
	if sessions.calls != 0 || sessions.readyForCalls != 0 {
		t.Fatalf("oversized character projection reached session issuer: %#v", sessions)
	}
}
