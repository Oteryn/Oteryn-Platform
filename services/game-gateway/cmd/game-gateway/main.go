package main

import (
	"context"
	"errors"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/config"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/gateway"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/httpapi"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/nativelogin"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/platform"
	"github.com/blakinio/oteryn-platform/services/game-gateway/internal/session"
)

func main() {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	cfg, err := config.Load()
	if err != nil {
		logger.Error("configuration_invalid")
		os.Exit(1)
	}

	httpClient := &http.Client{Timeout: cfg.RequestTimeout}
	platformClient := platform.NewClient(cfg.PlatformBaseURL, cfg.PlatformServiceToken, httpClient)
	// Native-only mode keeps sessions a nil interface so legacy logins fail closed.
	var sessionIssuer gateway.SessionIssuer
	if cfg.LegacySessionEnabled {
		sessionIssuer = session.NewClient(cfg.SessionBaseURL, cfg.SessionServiceToken, httpClient)
	}
	service := gateway.NewService(platformClient, sessionIssuer)
	api := httpapi.NewServer(service, cfg.Version, logger)
	if cfg.NativeLoginEnabled {
		issuer := nativelogin.NewIssuerClient(cfg.PlatformBaseURL, cfg.PlatformServiceToken, cfg.NativeAdmissionTimeout, nil)
		api.EnableNativeLogin(nativelogin.NewHandler(issuer, logger))
	}

	server := &http.Server{
		Addr:              cfg.ListenAddress,
		Handler:           api.Handler(),
		ReadHeaderTimeout: 5 * time.Second,
		WriteTimeout:      10 * time.Second,
		IdleTimeout:       60 * time.Second,
	}

	shutdownContext, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	go func() {
		<-shutdownContext.Done()
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := server.Shutdown(ctx); err != nil {
			logger.Error("shutdown_failed")
		}
	}()

	logger.Info("gateway_started", "version", cfg.Version, "listen_address", cfg.ListenAddress, "native_login_enabled", cfg.NativeLoginEnabled, "legacy_session_enabled", cfg.LegacySessionEnabled)
	if err := server.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
		logger.Error("gateway_stopped_unexpectedly")
		os.Exit(1)
	}
	logger.Info("gateway_stopped")
}
