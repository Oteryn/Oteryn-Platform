package config

import "testing"

func TestLoadRequiresServiceCredentialsAndValidDependencyURLs(t *testing.T) {
	t.Setenv("OTERYN_PLATFORM_BASE_URL", "https://platform.example.test")
	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "https://session.example.test")
	t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
	t.Setenv("GAME_SESSION_SERVICE_TOKEN", "session-token")
	t.Setenv("GATEWAY_REQUEST_TIMEOUT", "3s")

	cfg, err := Load()
	if err != nil {
		t.Fatalf("Load returned error: %v", err)
	}
	if cfg.PlatformBaseURL != "https://platform.example.test" || cfg.SessionBaseURL != "https://session.example.test" {
		t.Fatalf("unexpected dependency URLs: %#v", cfg)
	}
	if cfg.RequestTimeout.String() != "3s" {
		t.Fatalf("unexpected timeout: %s", cfg.RequestTimeout)
	}
}

func TestLoadAllowsHTTPOnlyForLoopbackDependencies(t *testing.T) {
	for _, raw := range []string{
		"http://127.0.0.1:8000",
		"http://localhost:8000",
		"http://[::1]:8000",
	} {
		t.Run(raw, func(t *testing.T) {
			t.Setenv("OTERYN_PLATFORM_BASE_URL", raw)
			t.Setenv("GAME_SESSION_SERVICE_BASE_URL", raw)
			t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
			t.Setenv("GAME_SESSION_SERVICE_TOKEN", "session-token")

			if _, err := Load(); err != nil {
				t.Fatalf("expected loopback URL %q to be accepted: %v", raw, err)
			}
		})
	}
}

func TestLoadRejectsHTTPForNonLoopbackDependencies(t *testing.T) {
	for _, raw := range []string{
		"http://platform.internal:8000",
		"http://10.0.0.10:8000",
		"http://192.168.1.10:8000",
	} {
		t.Run(raw, func(t *testing.T) {
			t.Setenv("OTERYN_PLATFORM_BASE_URL", raw)
			t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "https://session.example.test")
			t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
			t.Setenv("GAME_SESSION_SERVICE_TOKEN", "session-token")

			if _, err := Load(); err == nil {
				t.Fatalf("expected non-loopback HTTP URL %q to be rejected", raw)
			}
		})
	}
}

func TestLoadFailsClosedForMissingCredentials(t *testing.T) {
	t.Setenv("OTERYN_PLATFORM_BASE_URL", "https://platform.example.test")
	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "https://session.example.test")
	t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "")
	t.Setenv("GAME_SESSION_SERVICE_TOKEN", "")

	if _, err := Load(); err == nil {
		t.Fatal("expected missing service credentials to fail")
	}
}

func TestLoadRejectsCredentialBearingOrQueryDependencyURLs(t *testing.T) {
	for _, raw := range []string{
		"https://user:password@platform.example.test",
		"https://platform.example.test?token=secret",
		"file:///tmp/platform.sock",
	} {
		t.Run(raw, func(t *testing.T) {
			t.Setenv("OTERYN_PLATFORM_BASE_URL", raw)
			t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "https://session.example.test")
			t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
			t.Setenv("GAME_SESSION_SERVICE_TOKEN", "session-token")

			if _, err := Load(); err == nil {
				t.Fatalf("expected URL %q to be rejected", raw)
			}
		})
	}
}

func TestLoadRejectsUnboundedRequestTimeout(t *testing.T) {
	t.Setenv("OTERYN_PLATFORM_BASE_URL", "https://platform.example.test")
	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "https://session.example.test")
	t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
	t.Setenv("GAME_SESSION_SERVICE_TOKEN", "session-token")
	t.Setenv("GATEWAY_REQUEST_TIMEOUT", "2m")

	if _, err := Load(); err == nil {
		t.Fatal("expected excessive request timeout to fail")
	}
}

func TestLoadNativeLoginIsDefaultOffAndStrictlyParsed(t *testing.T) {
	t.Setenv("OTERYN_PLATFORM_BASE_URL", "https://platform.example.test")
	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "https://session.example.test")
	t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
	t.Setenv("GAME_SESSION_SERVICE_TOKEN", "session-token")

	cfg, err := Load()
	if err != nil || cfg.NativeLoginEnabled || cfg.NativeAdmissionTimeout.String() != "5s" {
		t.Fatalf("expected native login off with a 5s issuer timeout, got %#v err=%v", cfg, err)
	}

	t.Setenv("GATEWAY_NATIVE_LOGIN_ENABLED", "true")
	t.Setenv("GATEWAY_NATIVE_ADMISSION_TIMEOUT", "2s")
	cfg, err = Load()
	if err != nil || !cfg.NativeLoginEnabled || cfg.NativeAdmissionTimeout.String() != "2s" {
		t.Fatalf("expected native login on with a 2s issuer timeout, got %#v err=%v", cfg, err)
	}

	for _, invalid := range []string{"1", "TRUE", "yes", "on"} {
		t.Setenv("GATEWAY_NATIVE_LOGIN_ENABLED", invalid)
		if _, err := Load(); err == nil {
			t.Fatalf("GATEWAY_NATIVE_LOGIN_ENABLED=%q must be refused", invalid)
		}
	}
	t.Setenv("GATEWAY_NATIVE_LOGIN_ENABLED", "false")
	for _, invalid := range []string{"0s", "-1s", "9s", "soon"} {
		t.Setenv("GATEWAY_NATIVE_ADMISSION_TIMEOUT", invalid)
		if _, err := Load(); err == nil {
			t.Fatalf("GATEWAY_NATIVE_ADMISSION_TIMEOUT=%q must be refused", invalid)
		}
	}
}

func TestLoadNativeOnlyModeDisablesTheLegacySessionPath(t *testing.T) {
	t.Setenv("OTERYN_PLATFORM_BASE_URL", "https://platform.example.test")
	t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "")
	t.Setenv("GAME_SESSION_SERVICE_TOKEN", "")
	t.Setenv("GATEWAY_NATIVE_LOGIN_ENABLED", "true")

	cfg, err := Load()
	if err != nil {
		t.Fatalf("native-only configuration must load: %v", err)
	}
	if !cfg.NativeLoginEnabled || cfg.LegacySessionEnabled {
		t.Fatalf("expected native-only mode, got %#v", cfg)
	}
}

func TestLoadRejectsPartialSessionConfiguration(t *testing.T) {
	for _, native := range []string{"", "true"} {
		for _, partial := range []struct{ url, token string }{
			{url: "https://session.example.test", token: ""},
			{url: "", token: "session-token"},
		} {
			t.Run("native="+native+"/url="+partial.url, func(t *testing.T) {
				t.Setenv("OTERYN_PLATFORM_BASE_URL", "https://platform.example.test")
				t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
				t.Setenv("GAME_SESSION_SERVICE_BASE_URL", partial.url)
				t.Setenv("GAME_SESSION_SERVICE_TOKEN", partial.token)
				t.Setenv("GATEWAY_NATIVE_LOGIN_ENABLED", native)

				if _, err := Load(); err == nil {
					t.Fatal("exactly one GAME_SESSION_SERVICE_* variable must be refused")
				}
			})
		}
	}
}

func TestLoadLegacyModeStillRequiresTheSessionService(t *testing.T) {
	t.Setenv("OTERYN_PLATFORM_BASE_URL", "https://platform.example.test")
	t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "")
	t.Setenv("GAME_SESSION_SERVICE_TOKEN", "")
	t.Setenv("GATEWAY_NATIVE_LOGIN_ENABLED", "false")

	if _, err := Load(); err == nil {
		t.Fatal("legacy mode without the session service must fail")
	}

	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "https://session.example.test")
	t.Setenv("GAME_SESSION_SERVICE_TOKEN", "session-token")
	for _, native := range []string{"false", "true"} {
		t.Setenv("GATEWAY_NATIVE_LOGIN_ENABLED", native)
		cfg, err := Load()
		if err != nil || !cfg.LegacySessionEnabled {
			t.Fatalf("native=%s: expected the legacy session path enabled, got %#v err=%v", native, cfg, err)
		}
	}
}

func TestLoadNativeOnlyKeepsTheHTTPSRuleForNonLoopbackPlatform(t *testing.T) {
	t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "")
	t.Setenv("GAME_SESSION_SERVICE_TOKEN", "")
	t.Setenv("GATEWAY_NATIVE_LOGIN_ENABLED", "true")

	t.Setenv("OTERYN_PLATFORM_BASE_URL", "http://platform.internal:8000")
	if _, err := Load(); err == nil {
		t.Fatal("non-loopback http must be rejected in native-only mode")
	}

	t.Setenv("OTERYN_PLATFORM_BASE_URL", "https://platform.internal:8443")
	if _, err := Load(); err != nil {
		t.Fatalf("https to a non-loopback host must be accepted: %v", err)
	}
}

func TestLoadRejectsNonLoopbackHTTPSessionServiceAndAcceptsHTTPS(t *testing.T) {
	t.Setenv("OTERYN_PLATFORM_BASE_URL", "https://platform.example.test")
	t.Setenv("OTERYN_PLATFORM_SERVICE_TOKEN", "platform-token")
	t.Setenv("GAME_SESSION_SERVICE_TOKEN", "session-token")

	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "http://session.internal:8000")
	if _, err := Load(); err == nil {
		t.Fatal("non-loopback http session service must be rejected")
	}

	t.Setenv("GAME_SESSION_SERVICE_BASE_URL", "https://session.internal:8443")
	if _, err := Load(); err != nil {
		t.Fatalf("https session service on a non-loopback host must be accepted: %v", err)
	}
}
