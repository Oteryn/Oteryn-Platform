package gateway

import (
	"bytes"
	"encoding/json"
	"testing"
)

func TestGameplayOfferCandidateRejectsUnknownDuplicateAndNonCanonicalJSONFields(t *testing.T) {
	payload := mustMarshalJSON(t, validNativeLoginRequest())

	for _, test := range []struct {
		name        string
		replacement string
	}{
		{name: "unknown field", replacement: `"family":"oteryn","unexpected":true`},
		{name: "duplicate field", replacement: `"family":"oteryn","family":"oteryn"`},
		{name: "case variant profile", replacement: `"family":"oteryn","Profile":null`},
		{name: "case variant native version", replacement: `"family":"oteryn","NATIVE_PROTOCOL_VERSION":1`},
	} {
		t.Run(test.name, func(t *testing.T) {
			mutated := replaceFirstFamilyField(t, payload, test.replacement)
			var request LoginRequest
			if err := json.Unmarshal(mutated, &request); err == nil {
				t.Fatalf("expected strict candidate JSON rejection for %s", mutated)
			}
		})
	}
}

func TestGameplayPolicyCandidateRejectsUnknownDuplicateAndNonCanonicalJSONFields(t *testing.T) {
	payload := mustMarshalJSON(t, validNativePolicy())

	for _, test := range []struct {
		name        string
		replacement string
	}{
		{name: "unknown field", replacement: `"family":"oteryn","unexpected":true`},
		{name: "duplicate field", replacement: `"family":"oteryn","family":"oteryn"`},
		{name: "case variant profile", replacement: `"family":"oteryn","Profile":null`},
		{name: "case variant native version", replacement: `"family":"oteryn","NATIVE_PROTOCOL_VERSION":1`},
	} {
		t.Run(test.name, func(t *testing.T) {
			mutated := replaceFirstFamilyField(t, payload, test.replacement)
			var policy GameplayPolicy
			if err := json.Unmarshal(mutated, &policy); err == nil {
				t.Fatalf("expected strict policy candidate JSON rejection for %s", mutated)
			}
		})
	}
}

func mustMarshalJSON(t *testing.T, value any) []byte {
	t.Helper()
	payload, err := json.Marshal(value)
	if err != nil {
		t.Fatalf("marshal JSON: %v", err)
	}
	return payload
}

func replaceFirstFamilyField(t *testing.T, payload []byte, replacement string) []byte {
	t.Helper()
	anchor := []byte(`"family":"oteryn"`)
	if !bytes.Contains(payload, anchor) {
		t.Fatalf("missing native family field in %s", payload)
	}
	return bytes.Replace(payload, anchor, []byte(replacement), 1)
}
