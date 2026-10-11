package config

import (
	"fmt"
	"net"
	"net/url"
	"os"
	"strings"
	"time"
)

type Config struct {
	ListenAddress        string
	PlatformBaseURL      string
	PlatformServiceToken string
	SessionBaseURL       string
	SessionServiceToken  string
	RequestTimeout       time.Duration
	Version              string
	// NativeLoginEnabled routes protocol_version 2 logins to the Platform native admission
	// issuer. The built-in default stays off: the gateway cannot tell preproduction from
	// production, so each deployment opts in with GATEWAY_NATIVE_LOGIN_ENABLED=true (D963 Q1=A,
	// D607). The Canary path is unchanged either way.
	NativeLoginEnabled     bool
	NativeAdmissionTimeout time.Duration
	// LegacySessionEnabled is false only in native-only mode: native login enabled and both
	// GAME_SESSION_SERVICE_* variables unset. The gateway then builds no session client and
	// refuses legacy logins.
	LegacySessionEnabled bool
}

// maxNativeAdmissionTimeout keeps one issuer exchange inside the server write timeout.
const maxNativeAdmissionTimeout = 8 * time.Second

func Load() (Config, error) {
	cfg := Config{
		ListenAddress:        valueOrDefault("GATEWAY_LISTEN_ADDR", ":8080"),
		PlatformBaseURL:      strings.TrimRight(os.Getenv("OTERYN_PLATFORM_BASE_URL"), "/"),
		PlatformServiceToken: os.Getenv("OTERYN_PLATFORM_SERVICE_TOKEN"),
		SessionBaseURL:       strings.TrimRight(os.Getenv("GAME_SESSION_SERVICE_BASE_URL"), "/"),
		SessionServiceToken:  os.Getenv("GAME_SESSION_SERVICE_TOKEN"),
		Version:              valueOrDefault("GATEWAY_VERSION", "dev"),
		RequestTimeout:       5 * time.Second,

		NativeAdmissionTimeout: 5 * time.Second,
	}

	if raw := os.Getenv("GATEWAY_REQUEST_TIMEOUT"); raw != "" {
		parsed, err := time.ParseDuration(raw)
		if err != nil || parsed <= 0 || parsed > 30*time.Second {
			return Config{}, fmt.Errorf("invalid GATEWAY_REQUEST_TIMEOUT")
		}
		cfg.RequestTimeout = parsed
	}

	switch os.Getenv("GATEWAY_NATIVE_LOGIN_ENABLED") {
	case "", "false":
	case "true":
		cfg.NativeLoginEnabled = true
	default:
		return Config{}, fmt.Errorf("invalid GATEWAY_NATIVE_LOGIN_ENABLED")
	}
	if raw := os.Getenv("GATEWAY_NATIVE_ADMISSION_TIMEOUT"); raw != "" {
		parsed, err := time.ParseDuration(raw)
		if err != nil || parsed <= 0 || parsed > maxNativeAdmissionTimeout {
			return Config{}, fmt.Errorf("invalid GATEWAY_NATIVE_ADMISSION_TIMEOUT")
		}
		cfg.NativeAdmissionTimeout = parsed
	}

	if cfg.PlatformServiceToken == "" {
		return Config{}, fmt.Errorf("service credentials are required")
	}
	if err := validateBaseURL(cfg.PlatformBaseURL); err != nil {
		return Config{}, fmt.Errorf("invalid OTERYN_PLATFORM_BASE_URL: %w", err)
	}

	sessionURLSet, sessionTokenSet := cfg.SessionBaseURL != "", cfg.SessionServiceToken != ""
	switch {
	case !sessionURLSet && !sessionTokenSet && cfg.NativeLoginEnabled:
		cfg.LegacySessionEnabled = false
	case !sessionURLSet || !sessionTokenSet:
		return Config{}, fmt.Errorf("GAME_SESSION_SERVICE_BASE_URL and GAME_SESSION_SERVICE_TOKEN are required together")
	default:
		if err := validateBaseURL(cfg.SessionBaseURL); err != nil {
			return Config{}, fmt.Errorf("invalid GAME_SESSION_SERVICE_BASE_URL: %w", err)
		}
		cfg.LegacySessionEnabled = true
	}
	if strings.TrimSpace(cfg.ListenAddress) == "" {
		return Config{}, fmt.Errorf("GATEWAY_LISTEN_ADDR is empty")
	}

	return cfg, nil
}

func validateBaseURL(raw string) error {
	parsed, err := url.Parse(raw)
	if err != nil {
		return err
	}
	if parsed.Scheme != "http" && parsed.Scheme != "https" {
		return fmt.Errorf("scheme must be http or https")
	}
	if parsed.Host == "" || parsed.User != nil || parsed.RawQuery != "" || parsed.Fragment != "" {
		return fmt.Errorf("URL must contain only scheme, host and optional path")
	}
	if parsed.Scheme == "http" && !isLoopbackHost(parsed.Hostname()) {
		return fmt.Errorf("non-loopback dependencies must use https")
	}
	return nil
}

func isLoopbackHost(host string) bool {
	if strings.EqualFold(host, "localhost") {
		return true
	}
	ip := net.ParseIP(host)
	return ip != nil && ip.IsLoopback()
}

func valueOrDefault(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}
