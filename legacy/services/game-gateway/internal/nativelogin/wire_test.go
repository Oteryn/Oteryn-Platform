package nativelogin

import (
	"strings"
	"testing"
)

const (
	testTicket     = "native-ticket-secret-never-log"
	testAttemptRef = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b"
	testCharacter  = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c"
	testChannel    = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7d"
	testWorld      = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7e"
)

func validRequestJSON() string {
	return `{"protocol_version":2,"game_login_ticket":"` + testTicket + `","attempt_ref":"` + testAttemptRef +
		`","character_id":"` + testCharacter + `","channel_id":null,"offer":{"client_build":"0.1.0+abc123",` +
		`"client_platform":"windows","transports":[{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}]}}`
}

func TestParseRequestAcceptsExactContractRequestAndReencodesIt(t *testing.T) {
	request, refusal := ParseRequest([]byte(validRequestJSON()))
	if refusal != nil {
		t.Fatalf("unexpected refusal %s", refusal.Code)
	}
	if request.ChannelID != nil || request.AttemptRef != testAttemptRef || request.GameLoginTicket != testTicket {
		t.Fatalf("unexpected request %#v", request)
	}
	encoded, err := EncodeIssuerRequest(request)
	if err != nil {
		t.Fatalf("encode: %v", err)
	}
	if string(encoded) != validRequestJSON() {
		t.Fatalf("issuer body is not the canonical contract body:\n%s", encoded)
	}
	for _, rendered := range []string{request.String(), request.GoString()} {
		if strings.Contains(rendered, testTicket) {
			t.Fatalf("request rendering leaks the ticket: %s", rendered)
		}
	}
}

func TestParseRequestRejectsEverythingOutsideTheExactSchema(t *testing.T) {
	valid := validRequestJSON()
	transport := `{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}`
	cases := map[string]string{
		"empty":                ``,
		"not an object":        `[]`,
		"trailing value":       valid + `{}`,
		"unknown member":       strings.Replace(valid, `"channel_id":null`, `"channel_id":null,"extra":1`, 1),
		"duplicate member":     strings.Replace(valid, `"channel_id":null`, `"channel_id":null,"channel_id":null`, 1),
		"escaped duplicate":    strings.Replace(valid, `"channel_id":null`, `"channel_id":null,"channel_id":null`, 1),
		"missing member":       strings.Replace(valid, `"channel_id":null,`, ``, 1),
		"null ticket":          strings.Replace(valid, `"`+testTicket+`"`, `null`, 1),
		"null offer":           strings.Replace(valid, valid[strings.Index(valid, `{"client_build"`):len(valid)-1], `null`, 1),
		"ticket with space":    strings.Replace(valid, testTicket, "native ticket", 1),
		"ticket too long":      strings.Replace(valid, testTicket, strings.Repeat("t", 257), 1),
		"ticket non ascii":     strings.Replace(valid, testTicket, "ticketé", 1),
		"uppercase attempt":    strings.Replace(valid, testAttemptRef, strings.ToUpper(testAttemptRef), 1),
		"uuidv4 attempt":       strings.Replace(valid, testAttemptRef, "0192b3c4-5d6e-4f80-9a1b-2c3d4e5f6a7b", 1),
		"nil character":        strings.Replace(valid, testCharacter, "00000000-0000-0000-0000-000000000000", 1),
		"channel not uuidv7":   strings.Replace(valid, `"channel_id":null`, `"channel_id":"1"`, 1),
		"channel number":       strings.Replace(valid, `"channel_id":null`, `"channel_id":1`, 1),
		"unknown platform":     strings.Replace(valid, `"windows"`, `"linux"`, 1),
		"empty build":          strings.Replace(valid, `"0.1.0+abc123"`, `""`, 1),
		"no transports":        strings.Replace(valid, `[`+transport+`]`, `[]`, 1),
		"five transports":      strings.Replace(valid, `[`+transport+`]`, `[`+strings.Repeat(transport+`,`, 4)+transport+`]`, 1),
		"fractional major":     strings.Replace(valid, `"protocol_major":1`, `"protocol_major":1.0`, 1),
		"string major":         strings.Replace(valid, `"protocol_major":1`, `"protocol_major":"1"`, 1),
		"negative profile":     strings.Replace(valid, `"transport_profile":1`, `"transport_profile":-1`, 1),
		"unknown transport":    strings.Replace(valid, `"alpn":"oteryn-game/1"`, `"alpn":"oteryn-game/1","x":1`, 1),
		"too deep":             strings.Replace(valid, `"alpn":"oteryn-game/1"`, `"alpn":["oteryn-game/1"]`, 1),
		"oversized":            strings.Replace(valid, `"0.1.0+abc123"`, `"0.1.0+abc123"`+strings.Repeat(" ", MaxRequestBytes), 1),
		"offer member missing": strings.Replace(valid, `"client_build":"0.1.0+abc123",`, ``, 1),
	}
	for name, body := range cases {
		t.Run(name, func(t *testing.T) {
			if _, refusal := ParseRequest([]byte(body)); refusal == nil || refusal.Code != CodeRequestMalformed {
				t.Fatalf("expected %s, got %#v", CodeRequestMalformed, refusal)
			}
		})
	}
}

func TestParseRequestMapsVersionAndOfferRefusals(t *testing.T) {
	valid := validRequestJSON()
	if _, refusal := ParseRequest([]byte(strings.Replace(valid, `"protocol_version":2`, `"protocol_version":3`, 1))); refusal == nil || refusal.Code != CodeUnsupportedVersion {
		t.Fatalf("expected unsupported version, got %#v", refusal)
	}
	unsupported := strings.Replace(valid, `"alpn":"oteryn-game/1"`, `"alpn":"oteryn-game/2"`, 1)
	if _, refusal := ParseRequest([]byte(unsupported)); refusal == nil || refusal.Code != CodeOfferUnsupported {
		t.Fatalf("expected offer unsupported, got %#v", refusal)
	}
	mixed := strings.Replace(valid, `[{`, `[{"protocol_major":2,"transport_profile":1,"alpn":"oteryn-game/2"},{`, 1)
	if request, refusal := ParseRequest([]byte(mixed)); refusal != nil || len(request.Offer.Transports) != 2 {
		t.Fatalf("an unknown transport beside a supported one must be kept and ignored for selection: %#v", refusal)
	}
	withChannel := strings.Replace(valid, `"channel_id":null`, `"channel_id":"`+testChannel+`"`, 1)
	if request, refusal := ParseRequest([]byte(withChannel)); refusal != nil || request.ChannelID == nil || *request.ChannelID != testChannel {
		t.Fatalf("expected requested channel, got %#v", refusal)
	}
}

func TestIsNativeRequestSelectsOnlyIntegerVersionTwo(t *testing.T) {
	for body, want := range map[string]bool{
		validRequestJSON():                                        true,
		`{"protocol_version":2}`:                                  true,
		`{"protocol_version":1,"game_login_ticket":"t"}`:          false,
		`{"protocol_version":2.0}`:                                false,
		`{"protocol_version":"2"}`:                                false,
		`{"game_login_ticket":"t"}`:                               false,
		`not json`:                                                false,
		`{"protocol_version":1,"protocol_version":2}`:             true,
		`{"protocol_version":2,"offer":{"protocol_version":1}}`:   true,
		`{"offer":{"protocol_version":2},"protocol_version":1}  `: false,
	} {
		if got := IsNativeRequest([]byte(body)); got != want {
			t.Fatalf("IsNativeRequest(%s) = %v, want %v", body, got, want)
		}
	}
}

func TestAttemptRefEchoOnlyForCanonicalValues(t *testing.T) {
	if echo := AttemptRefEcho([]byte(validRequestJSON())); echo == nil || *echo != testAttemptRef {
		t.Fatalf("expected echo, got %v", echo)
	}
	for _, body := range []string{
		`{"attempt_ref":"` + strings.ToUpper(testAttemptRef) + `"}`,
		`{"attempt_ref":1}`,
		`{}`,
		`garbage`,
	} {
		if echo := AttemptRefEcho([]byte(body)); echo != nil {
			t.Fatalf("expected no echo for %s", body)
		}
	}
}

func TestPublicErrorMappingMatchesContractTable(t *testing.T) {
	want := map[string][2]any{
		CodeRequestMalformed:       {400, "RETRY_LOGIN"},
		CodeUnsupportedVersion:     {400, "CLIENT_UPDATE_REQUIRED"},
		CodeOfferUnsupported:       {409, "CLIENT_UPDATE_REQUIRED"},
		CodeAuthenticationRequired: {401, "AUTHENTICATION_REQUIRED"},
		CodeAttemptConflict:        {409, "RETRY_LOGIN"},
		CodeCharacterConflict:      {409, "SESSION_UNAVAILABLE"},
		CodeRouteUnavailable:       {503, "TEMPORARILY_UNAVAILABLE"},
		CodeRateLimited:            {429, "TEMPORARILY_UNAVAILABLE"},
		CodeReconciliationRequired: {503, "TEMPORARILY_UNAVAILABLE"},
		CodeGrantExpired:           {409, "RETRY_LOGIN"},
		CodeUnavailable:            {503, "TEMPORARILY_UNAVAILABLE"},
	}
	if len(want) != len(publicErrors) {
		t.Fatalf("public error table has %d codes, want %d", len(publicErrors), len(want))
	}
	for code, expected := range want {
		spec := publicErrors[code]
		if spec.status != expected[0] || spec.publicClass != expected[1] {
			t.Fatalf("%s maps to %d/%s", code, spec.status, spec.publicClass)
		}
	}
}
